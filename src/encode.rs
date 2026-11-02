//! # Encoding
//!
//! Quoted-printable encoding of a MIME body, in lines of at most 76
//! characters soft break included (RFC 2045 6.7).
//!
//! In text mode, LF and CRLF become hard line breaks; in binary mode, CR and
//! LF are escaped like any other byte and the output has soft line breaks
//! only. `=`, bytes from 0x7F up, control characters other than TAB, a bare
//! CR in text mode and a space or tab that would end a line are escaped as
//! `=XX`, uppercase; an escape is never split across lines. In Q mode, the
//! RFC 2047 encoding of header words, letters, digits and `!*+-/` stay
//! literal, a space becomes `_`, every other byte is escaped, and no line
//! breaks at all: the caller splits the text into encoded words, which
//! [`QuotedPrintableCodec::encoded_len`] sizes exactly.
//!
//! [`QuotedPrintableEncode`] takes the input chunk by chunk; the one-shot
//! [`QuotedPrintableCodec::encode`] and `QuotedPrintableCodec::encode_vec`
//! encode a whole input. Decoding is the sibling module
//! [`decode`](crate::decode).

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
use crate::neon;
#[cfg(target_arch = "x86_64")]
use crate::{avx2, avx512};
use crate::{
    codec::{QuotedPrintableCodec, QuotedPrintableMode},
    edge::Edge,
    is_blank, is_plain, mask, pad, window,
};

/// The most characters a line holds before its soft break `=`.
pub(crate) const LINE: usize = 75;

/// The output bytes a block may touch: a portable block commits at most 32
/// escapes and two soft breaks, under 128 bytes, and copies a run whole past
/// them; an AVX-512 chunk writes three groups of at most 66 bytes, and up to
/// 68 bytes past the start of each; an AVX2 or NEON group writes at most 51
/// bytes, and up to 99 past its start.
pub(crate) const ROOM: usize = 200;

/// Runs the SIMD encoder `$kernel`, built for the mode and the line ending of
/// `$codec`.
#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "aarch64", target_feature = "neon")
))]
macro_rules! kernel {
    ($($kernel:ident)::+, $codec:expr, $($arg:expr),+) => {{
        use crate::codec::{QuotedPrintableEol, QuotedPrintableMode};

        let binary = $codec.mode == QuotedPrintableMode::Binary;

        match (binary, $codec.eol) {
            (false, QuotedPrintableEol::Crlf) => $($kernel)::+::<true, true>($($arg),+),
            (false, QuotedPrintableEol::Lf) => $($kernel)::+::<true, false>($($arg),+),
            (true, QuotedPrintableEol::Crlf) => $($kernel)::+::<false, true>($($arg),+),
            (true, QuotedPrintableEol::Lf) => $($kernel)::+::<false, false>($($arg),+),
        }
    }};
}

/// I/O-free quoted-printable encoder, resumed with the caller's input and
/// output buffers.
///
/// The input can be cut anywhere: the encoder carries a trailing space or
/// tab, and a trailing CR in text mode, until the next byte tells whether a
/// line ends there.
///
/// ```
/// use quoted_printable_codec::{codec::QuotedPrintableCodec, encode::QuotedPrintableEncode};
///
/// let mut encoder = QuotedPrintableEncode::new(QuotedPrintableCodec::default());
/// let mut out = [0; 32];
///
/// let first = encoder.resume(b"a = b ", &mut out);
/// let second = encoder.resume(b"\r\nc ", &mut out[first.written..]);
/// let written = first.written + second.written;
/// let end = encoder.finish(&mut out[written..]).unwrap();
///
/// assert_eq!(&out[..written + end], b"a =3D b=20\r\nc=20");
/// ```
///
/// Refs: <https://datatracker.ietf.org/doc/html/rfc2045#section-6.7>
#[derive(Clone, Debug)]
pub struct QuotedPrintableEncode {
    codec: QuotedPrintableCodec,
    column: usize,
    // NOTE: carries a block, queues what a block touches.
    edge: Edge<32, ROOM>,
}

impl QuotedPrintableEncode {
    /// Builds an encoder running the backend, the mode and the line ending of
    /// `codec`.
    pub fn new(codec: QuotedPrintableCodec) -> Self {
        Self {
            codec,
            column: 0,
            edge: Edge::default(),
        }
    }

    /// Encodes as much of `src` as fits in `dst`.
    ///
    /// The caller drains `dst` and resumes with the rest of `src` or the
    /// next chunk; one byte of room always lets the encoder progress. The
    /// bytes of `dst` past the ones written may be overwritten.
    pub fn resume(&mut self, src: &[u8], dst: &mut [u8]) -> QuotedPrintableEncodeOutput {
        let (codec, column) = (self.codec, &mut self.column);
        let (read, written) = self.edge.resume(src, dst, |src, dst, last| {
            blocks(src, dst, last, codec, column)
        });

        QuotedPrintableEncodeOutput { read, written }
    }

    /// Ends the input: writes what is still queued and the carried space,
    /// tab or CR, escaped as the end of the input asks.
    ///
    /// Returns the bytes written, the encoder starting over for another
    /// input, or none, writing nothing, when `dst` is too short.
    pub fn finish(&mut self, dst: &mut [u8]) -> Option<usize> {
        let mut column = self.column;
        let written = self.edge.finish(dst, |src, dst| {
            blocks(src, dst, true, self.codec, &mut column)[1]
        })?;
        self.column = 0;
        Some(written)
    }
}

/// What an encoding call did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QuotedPrintableEncodeOutput {
    /// The input bytes consumed.
    pub read: usize,
    /// The output bytes written.
    pub written: usize,
}

/// One-shot encoding.
impl QuotedPrintableCodec {
    /// Encodes the whole of `src` into `dst`, returning the bytes written.
    ///
    /// Returns none, writing nothing, when `dst` is shorter than
    /// [`encoded_len_max`](Self::encoded_len_max) asks. Within that length,
    /// the bytes of `dst` past the ones written may be overwritten.
    #[inline]
    pub fn encode(self, src: &[u8], dst: &mut [u8]) -> Option<usize> {
        let dst = dst.get_mut(..self.encoded_len_max(src.len())?)?;
        Some(blocks(src, dst, true, self, &mut 0)[1])
    }

    /// Encodes the whole of `src` at the end of `out`.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    pub fn encode_vec(self, src: &[u8], out: &mut Vec<u8>) {
        let start = out.len();
        let len = self.encoded_len_max(src.len()).unwrap_or(usize::MAX);
        out.resize(start + len, 0);

        let written = self.encode(src, &mut out[start..]).unwrap_or(0);
        out.truncate(start + written);
    }

    /// The room [`encode`](Self::encode) needs for an input of `len` bytes,
    /// whatever they hold, or none when that overflows `usize`.
    ///
    /// Every byte takes at most three characters, and a soft break follows
    /// at least 73 characters, so at least 25 input bytes; past them, the
    /// room a block writes ahead of what it commits.
    pub fn encoded_len_max(self, len: usize) -> Option<usize> {
        let breaks = (len / 25 + 1).checked_mul(self.eol.soft_break().1)?;
        len.checked_mul(3)?.checked_add(breaks + ROOM)
    }

    /// The exact number of bytes [`encode`](Self::encode) writes for `src`:
    /// counted in Q mode, encoded into a scratch buffer in the others.
    pub fn encoded_len(self, src: &[u8]) -> usize {
        if self.mode == QuotedPrintableMode::Q {
            let width = |&byte: &u8| {
                if is_phrase(byte) || byte == b' ' {
                    1
                } else {
                    3
                }
            };
            return src.iter().map(width).sum();
        }

        let mut encoder = QuotedPrintableEncode::new(self);
        // NOTE: what `finish` writes, its queue and the carry it encodes, fits
        // in twice `ROOM`.
        let mut scratch = [0; 2 * ROOM];
        let mut src = src;
        let mut len = 0;

        while !src.is_empty() {
            let output = encoder.resume(src, &mut scratch);
            src = &src[output.read..];
            len += output.written;
        }

        len + encoder.finish(&mut scratch).unwrap_or(0)
    }
}

/// Encodes `src` into `dst` from `column`, as much as fits, with the blocks of
/// the codec's backend, returning the bytes read and written, and none held,
/// as [`Edge`] asks.
///
/// Unless `last` tells that the input ends with `src`, a space, tab or CR
/// that may end a line is left unread when `src` ends before telling.
fn blocks(
    src: &[u8],
    dst: &mut [u8],
    last: bool,
    codec: QuotedPrintableCodec,
    column: &mut usize,
) -> [usize; 3] {
    match (codec.mode, codec.cpu.backend()) {
        (QuotedPrintableMode::Q, _) => words(src, dst),
        #[cfg(target_arch = "x86_64")]
        // SAFETY: the codec's token names the AVX-512 backend, which the CPU
        // supports.
        (_, crate::cpu::QuotedPrintableBackend::Avx512) if unsafe { avx512::dense(src) } => unsafe {
            kernel!(avx512::encode, codec, src, dst, last, column)
        },
        #[cfg(target_arch = "x86_64")]
        // SAFETY: the codec's token names the AVX2 backend, which the CPU
        // supports.
        (_, crate::cpu::QuotedPrintableBackend::Avx2) => unsafe {
            kernel!(avx2::encode, codec, src, dst, last, column)
        },
        #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
        // SAFETY: the codec's token names the NEON backend, which the CPU
        // supports.
        (_, crate::cpu::QuotedPrintableBackend::Neon) => unsafe {
            kernel!(neon::encode, codec, src, dst, last, column)
        },
        (QuotedPrintableMode::Binary, _) => walk::<false>(src, dst, last, codec, column),
        _ => walk::<true>(src, dst, last, codec, column),
    }
}

/// Encodes `src` into `dst` in the Q encoding, as much as fits, returning the
/// bytes read and written, and none held: every byte is a token of its own.
fn words(src: &[u8], dst: &mut [u8]) -> [usize; 3] {
    let mut written = 0;

    for (read, &byte) in src.iter().enumerate() {
        let (token, len) = match byte {
            b' ' => ([b'_'; 4], 1),
            _ if is_phrase(byte) => ([byte; 4], 1),
            _ => (ESCAPES[byte as usize], 3),
        };

        let Some(out) = dst.get_mut(written..written + len) else {
            return [read, written, 0];
        };

        out.copy_from_slice(&token[..len]);
        written += len;
    }

    [src.len(), written, 0]
}

/// The bytes the Q encoding writes literally: letters, digits and `!*+-/`,
/// the ones RFC 2047 5(3) allows in a phrase, so that a word goes anywhere.
const fn is_phrase(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'!' | b'*' | b'+' | b'-' | b'/')
}

/// The soft break a group of the AVX2 or NEON encoder needs, if any, and the
/// column after the group, from the bytes it escapes, the line feeds it
/// breaks on and the CRs it drops, a bit each, the length of its output and
/// the column before it; `crlf` tells that its line breaks are CRLF.
///
/// The literal or escape covering the line's first character past its room
/// takes the break before it, unless a line break comes first. A group has
/// at most 16 bytes, so its output fits in a line after one break.
#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "aarch64", target_feature = "neon")
))]
pub(crate) fn line(
    [esc, lf, drop]: [u32; 3],
    len: usize,
    col: usize,
    crlf: bool,
) -> (Option<usize>, usize) {
    // NOTE: the output offset of byte `i`: a character for each byte before
    // it, two more for each escape, one more for each CRLF line break, none
    // for each CR dropped.
    let at = |i: u32| {
        let count = |mask: u32| (mask & !(u32::MAX << i)).count_ones() as usize;
        i as usize + 2 * count(esc) + crlf as usize * count(lf) - count(drop)
    };
    let room = LINE - col;
    let need = room < len && (lf == 0 || at(lf.trailing_zeros()) > room);
    let cut = need.then(|| {
        let starts = (0..).take_while(|&i| at(i) <= room);
        starts.filter(|&i| drop >> i & 1 == 0).last().map_or(0, at)
    });
    let col = match (lf, cut) {
        (0, Some(cut)) => len - cut,
        (0, None) => col + len,
        _ => len - at(31 - lf.leading_zeros()) - 1 - crlf as usize,
    };

    (cut, col)
}

/// The portable loop of [`blocks`], in blocks of 32 bytes reading the 2 bytes
/// after them ahead, built apart for each mode, `TEXT` reading line breaks: a
/// mode tested for each byte costs a fifth of binary encoding.
fn walk<const TEXT: bool>(
    src: &[u8],
    dst: &mut [u8],
    last: bool,
    codec: QuotedPrintableCodec,
    column: &mut usize,
) -> [usize; 3] {
    // NOTE: a local, as the closure below compiles worse reading `TEXT`.
    let text = TEXT;
    let (soft, soft_len) = codec.eol.soft_break();
    // NOTE: padded first, so that its stores are done when blocks load them.
    let mut end = [0; 128];
    pad(&mut end, src);

    let mut read = 0;
    let mut written = 0;
    let mut col = *column;

    while let Some(out) = dst[written..].first_chunk_mut::<ROOM>() {
        let rest = src.len() - read;
        let w = window(&src[read..], &end);
        let mut cut = rest.min(34);

        if !last && text && w[..cut].ends_with(b"\r") {
            cut -= 1;
        }

        if !last && w[..cut].last().is_some_and(|&byte| is_blank(byte)) {
            cut -= 1;
        }

        let len = cut.min(32);

        if len == 0 {
            break;
        }

        let breaks = |at: usize| text && (w[at] == b'\n' || w[at..].starts_with(b"\r\n"));
        let end = if rest > len { breaks(len) } else { last };
        // NOTE: bit `len` stands for the block's end.
        let mut events = !mask(w, is_plain) as u64 & !(u64::MAX << len) | 1 << len;

        if end && is_blank(w[len - 1]) {
            events |= 1 << (len - 1);
        }

        let mut from = 0;
        // NOTE: input byte `i` goes to output offset `i + off - 128`, and the
        // line is full at output offset `lim - 128`: biased, both stay
        // positive when CRs are dropped, and the bias vanishes under `& 127`.
        let mut off = 128;
        let mut lim = 128 + LINE - col;

        while events != 0 {
            let at = events.trailing_zeros() as usize & 63;
            let line = at < len && breaks(at);

            if line && at > from && is_blank(w[at - 1]) {
                events |= 1 << (at - 1);
                continue;
            }

            let n = at + off;

            if n + 3 > lim && (n > lim || !line && at < len) {
                let cut = n.min(lim);
                out[(from + off) & 127..][..32].copy_from_slice(&w[from & 31..][..32]);
                out[cut & 127..][..4].copy_from_slice(&soft);
                from = cut - off;
                off += soft_len;
                lim = cut + soft_len + LINE;
            }

            out[(from + off) & 127..][..32].copy_from_slice(&w[from & 31..][..32]);

            // NOTE: the end's bit takes an escape too, past the output.
            if !line {
                out[(at + off) & 127..][..4].copy_from_slice(&ESCAPES[w[at] as usize]);
                off += 2;
            } else if w[at] == b'\n' {
                out[(at + off) & 127..][..3].copy_from_slice(&soft[1..]);
                off += soft_len - 2;
                lim = at + off + 1 + LINE;
            } else {
                off -= 1;
            }

            from = at + 1;
            events &= events - 1;
        }

        col = len + off - 2 + LINE - lim;
        read += len;
        written += len + off - 130;
    }

    *column = col;
    [read, written, 0]
}

/// The escape of each byte: `=` and two uppercase hex digits.
static ESCAPES: [[u8; 4]; 256] = {
    let mut table = [[0; 4]; 256];
    let mut byte = 0;

    while byte < 256 {
        let digits = b"0123456789ABCDEF";
        table[byte] = [b'=', digits[byte >> 4], digits[byte & 0xf], 0];
        byte += 1;
    }

    table
};

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};

    use simd_cpu::SimdCpu;

    use crate::{
        codec::QuotedPrintableEol,
        cpu::{QuotedPrintableCpu, available},
        encode::*,
    };

    const TEXT: QuotedPrintableCodec = QuotedPrintableCodec {
        cpu: QuotedPrintableCpu::new(SimdCpu::BASELINE),
        eol: QuotedPrintableEol::Crlf,
        mode: QuotedPrintableMode::Text,
    };

    const BINARY: QuotedPrintableCodec = QuotedPrintableCodec {
        mode: QuotedPrintableMode::Binary,
        ..TEXT
    };

    const Q: QuotedPrintableCodec = QuotedPrintableCodec {
        mode: QuotedPrintableMode::Q,
        ..TEXT
    };

    /// The encoding of `input` under every backend, checked the same, and as
    /// long as `encoded_len` tells.
    fn encoded(input: &[u8], codec: QuotedPrintableCodec) -> Vec<u8> {
        let outs: Vec<_> = available()
            .into_iter()
            .map(|cpu| {
                let codec = QuotedPrintableCodec { cpu, ..codec };
                let mut out = vec![0; codec.encoded_len_max(input.len()).unwrap()];
                let written = codec.encode(input, &mut out).unwrap();
                assert_eq!(codec.encoded_len(input), written);
                out.truncate(written);
                out
            })
            .collect();

        assert!(outs.windows(2).all(|pair| pair[0] == pair[1]));
        outs[0].clone()
    }

    fn text(input: &[u8]) -> Vec<u8> {
        encoded(input, TEXT)
    }

    #[test]
    fn printable_ascii_stays_literal() {
        assert_eq!(text(b"Hello, world!"), b"Hello, world!");
        assert_eq!(text(b"a=b"), b"a=3Db");
        assert_eq!(text("é".as_bytes()), b"=C3=A9");
        assert_eq!(text(b"\x00\x7f"), b"=00=7F");
    }

    #[test]
    fn line_breaks_become_crlf() {
        assert_eq!(text(b"a\nb\r\nc"), b"a\r\nb\r\nc");
        assert_eq!(text(b"a\rb"), b"a=0Db");
        assert_eq!(text(b"a\r"), b"a=0D");

        let lf = QuotedPrintableCodec {
            eol: QuotedPrintableEol::Lf,
            ..TEXT
        };

        assert_eq!(encoded(b"a\r\nb", lf), b"a\nb");
    }

    #[test]
    fn line_final_blanks_are_escaped() {
        assert_eq!(text(b"a \nb\t\r\nc "), b"a=20\r\nb=09\r\nc=20");
        assert_eq!(text(b"a  b"), b"a  b");
        assert_eq!(text(b"a \rb"), b"a =0Db");
        assert_eq!(text(b"a \r"), b"a =0D");
    }

    #[test]
    fn binary_escapes_line_breaks() {
        assert_eq!(encoded(b"a\r\nb \n", BINARY), b"a=0D=0Ab =0A");
    }

    #[test]
    fn q_keeps_only_the_phrase_set() {
        assert_eq!(encoded("a b,é?".as_bytes(), Q), b"a_b=2C=C3=A9=3F");
        assert_eq!(
            encoded(b"Az09!*+-/_=. \t\r\n", Q),
            b"Az09!*+-/=5F=3D=2E_=09=0D=0A"
        );
        assert_eq!(encoded(&[b'x'; 200], Q), [b'x'; 200]);
    }

    #[test]
    fn long_lines_get_soft_breaks() {
        let out = text(&[b'x'; 80]);
        assert_eq!(out.len(), 83);
        assert_eq!(&out[75..78], b"=\r\n");

        let out = text(&[0xff; 30]);
        let first = out.split(|&b| b == b'\n').next().unwrap();
        assert_eq!(first.len(), 76 + 1);
        assert!(first.ends_with(b"=FF=\r"));
    }

    #[test]
    fn the_length_bound_holds() {
        for len in 0..200 {
            for byte in [b'x', b' ', b'\n', 0xff] {
                for codec in [TEXT, BINARY, Q] {
                    let input = vec![byte; len];
                    let out = encoded(&input, codec);
                    assert!(out.len() <= codec.encoded_len_max(len).unwrap());
                }
            }
        }
    }

    #[test]
    fn one_byte_of_room_always_progresses() {
        let input: Vec<u8> = (0..=255).chain(b"a \r\nb \rc\t".iter().copied()).collect();

        for codec in available()
            .iter()
            .flat_map(|&cpu| [TEXT, BINARY, Q].map(|c| QuotedPrintableCodec { cpu, ..c }))
        {
            let mut encoder = QuotedPrintableEncode::new(codec);
            let mut src = &input[..];
            let mut out = Vec::new();
            let mut room = [0; 1];

            while !src.is_empty() {
                let output = encoder.resume(src, &mut room);
                assert!(output.read > 0 || output.written > 0);
                out.extend_from_slice(&room[..output.written]);
                src = &src[output.read..];
            }

            let mut tail = [0; ROOM];
            let end = encoder.finish(&mut tail).unwrap();
            out.extend_from_slice(&tail[..end]);
            assert_eq!(out, encoded(&input, codec));
        }
    }

    /// Once `resume` has read and written all it can, `finish` writes the
    /// space, tab or CR held at most, escaped, and a soft break: 9 bytes.
    #[test]
    fn finish_writes_the_held_bytes_only() {
        let input: Vec<u8> = b"x\r=\xff \t".iter().copied().cycle().take(70).collect();

        for codec in available()
            .iter()
            .flat_map(|&cpu| [TEXT, BINARY, Q].map(|c| QuotedPrintableCodec { cpu, ..c }))
        {
            for len in 0..input.len() {
                let mut encoder = QuotedPrintableEncode::new(codec);
                let mut src = &input[..len];
                let mut room = [0; 1];

                while !src.is_empty() {
                    src = &src[encoder.resume(src, &mut room).read..];
                }

                while encoder.resume(&[], &mut room).written > 0 {}
                assert!(encoder.finish(&mut [0; 9]).is_some(), "{codec:?} {len}");
            }
        }
    }

    #[test]
    fn a_short_buffer_writes_nothing() {
        let mut out = [0xaa; 3];
        assert_eq!(TEXT.encode(b"f", &mut out), None);
        assert_eq!(out, [0xaa; 3]);

        for cpu in available() {
            let mut encoder = QuotedPrintableEncode::new(QuotedPrintableCodec { cpu, ..TEXT });
            assert_eq!(encoder.resume(b" ", &mut []).read, 1);
            assert_eq!(encoder.finish(&mut out[..2]), None);
            assert_eq!(encoder.finish(&mut out), Some(3));
            assert_eq!(&out, b"=20");
        }
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn a_vec_is_appended_to() {
        let mut out = b"> ".to_vec();
        TEXT.encode_vec(b"a=b", &mut out);
        assert_eq!(out, b"> a=3Db");
    }
}
