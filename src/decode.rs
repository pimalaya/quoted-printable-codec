//! # Decoding
//!
//! Lenient quoted-printable decoding of a MIME body, as RFC 2045 6.7 asks of
//! robust implementations. Escapes decode in either case, soft breaks join,
//! trailing blanks go, CRLF and LF stay, a bare CR stays and is reported, a
//! malformed escape keeps its `=`. Decoding never fails: it returns the bytes
//! and whether the input was clean.
//!
//! Q mode (RFC 2047) reads `_` as a space and escapes in either case, and
//! keeps other bytes. Only a malformed escape makes it unclean, as words in
//! the wild carry raw bytes and lowercase digits.
//!
//! [`QuotedPrintableDecode`] takes the input chunk by chunk;
//! [`QuotedPrintableCodec::decode`] takes it whole.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
use crate::neon;
#[cfg(target_arch = "x86_64")]
use crate::{avx2, avx512};
use crate::{
    codec::{QuotedPrintableCodec, QuotedPrintableMode},
    cpu::QuotedPrintableBackend,
    edge::Edge,
    is_blank, is_plain, mask, pad, window,
};

/// The most trailing blanks a line break deletes; older ones stay as data.
const BLANKS_MAX: usize = 76;

/// The longest undecided token (`=`, the most blanks, a CR), plus one byte.
const CARRY: usize = BLANKS_MAX + 4;

/// I/O-free quoted-printable decoder, resumed with the caller's buffers.
///
/// The input can be cut anywhere: an unfinished escape or soft break, a CR
/// and trailing blanks are carried to the next call.
///
/// ```
/// use quoted_printable_codec::{codec::QuotedPrintableCodec, decode::QuotedPrintableDecode};
///
/// let mut decoder = QuotedPrintableDecode::new(QuotedPrintableCodec::default());
/// let mut out = [0; 16];
///
/// let first = decoder.resume(b"caf=C", &mut out);
/// let second = decoder.resume(b"3=A9 =\r\n!  \r\n", &mut out[first.written..]);
/// let written = first.written + second.written;
/// let end = decoder.finish(&mut out[written..]).unwrap();
///
/// assert_eq!(&out[..written + end.written], "café !\r\n".as_bytes());
/// assert!(end.clean);
/// ```
///
/// Refs: <https://datatracker.ietf.org/doc/html/rfc2045#section-6.7>
#[derive(Clone, Debug, Default)]
pub struct QuotedPrintableDecode {
    codec: QuotedPrintableCodec,
    edge: Edge<CARRY, CARRY>,
    dirty: bool,
}

impl QuotedPrintableDecode {
    /// Builds a decoder running `codec`, whose line ending it ignores.
    pub fn new(codec: QuotedPrintableCodec) -> Self {
        // NOTE: not `..Self::default()`, which reads the installed token.
        Self {
            codec,
            edge: Edge::default(),
            dirty: false,
        }
    }

    /// Decodes as much of `src` as fits in `dst`.
    ///
    /// The caller drains `dst` and resumes with the rest; one byte of room
    /// always progresses. Bytes past the written ones may be overwritten.
    pub fn resume(&mut self, src: &[u8], dst: &mut [u8]) -> QuotedPrintableDecodeOutput {
        let (codec, dirty) = (self.codec, &mut self.dirty);
        let (read, written) = self.edge.resume(src, dst, |src, dst, last| {
            tokens(codec, &src[..src.len().min(dst.len())], dst, last, dirty)
        });

        QuotedPrintableDecodeOutput {
            read,
            written,
            clean: !self.dirty,
        }
    }

    /// Ends the input: writes the queue, an unfinished escape and a last CR,
    /// and drops trailing blanks.
    ///
    /// Returns none, writing nothing, when `dst` is too short; otherwise
    /// resets the decoder.
    pub fn finish(&mut self, dst: &mut [u8]) -> Option<QuotedPrintableDecodeOutput> {
        let mut dirty = self.dirty;
        let written = self.edge.finish(dst, |src, dst| {
            tokens(self.codec, src, dst, true, &mut dirty)[1]
        })?;
        self.dirty = false;

        Some(QuotedPrintableDecodeOutput {
            read: 0,
            written,
            clean: !dirty,
        })
    }
}

/// What a decoding call did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QuotedPrintableDecodeOutput {
    /// The input bytes consumed.
    pub read: usize,
    /// The output bytes written.
    pub written: usize,
    /// Whether everything read so far was well-formed.
    pub clean: bool,
}

/// One-shot decoding.
impl QuotedPrintableCodec {
    /// Decodes the whole of `src` into `dst`.
    ///
    /// Returns none, writing nothing, when `dst` is shorter than
    /// [`decoded_len_max`](Self::decoded_len_max). Bytes past the written
    /// ones may be overwritten.
    #[inline]
    pub fn decode(self, src: &[u8], dst: &mut [u8]) -> Option<QuotedPrintableDecodeOutput> {
        if dst.len() < self.decoded_len_max(src.len()) {
            return None;
        }

        let mut dirty = false;
        let [read, written, _] = tokens(self, src, dst, true, &mut dirty);

        Some(QuotedPrintableDecodeOutput {
            read,
            written,
            clean: !dirty,
        })
    }

    /// Decodes the whole of `src` at the end of `out`, returning whether it
    /// was well-formed.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    pub fn decode_vec(self, src: &[u8], out: &mut Vec<u8>) -> bool {
        let start = out.len();
        out.resize(start + self.decoded_len_max(src.len()), 0);
        let output = self.decode(src, &mut out[start..]).unwrap();
        out.truncate(start + output.written);
        output.clean
    }

    /// The most bytes [`decode`](Self::decode) writes for `len` input bytes:
    /// each output byte takes one input byte at least.
    pub const fn decoded_len_max(self, len: usize) -> usize {
        len
    }
}

/// Decodes `src` into `dst`, at least as long, with the codec's backend or
/// the Q loop, returning the bytes read, written and held.
///
/// Unless `last`, a cut escape, soft break or CR is left unread, and blanks
/// a line break would delete are held, written past the counted ones.
fn tokens(
    codec: QuotedPrintableCodec,
    src: &[u8],
    dst: &mut [u8],
    last: bool,
    dirty: &mut bool,
) -> [usize; 3] {
    if codec.mode == QuotedPrintableMode::Q {
        return words(src, dst, last, dirty);
    }

    let cpu = codec.cpu;
    let mut read = 0;
    let mut written = 0;
    let mut blanks = 0;
    // NOTE: padded first, so its stores land before blocks load them.
    let mut end = [0; 128];

    if cpu.backend() == QuotedPrintableBackend::Portable {
        pad(&mut end, src);
    }

    loop {
        let (rest, out, prev) = (&src[read..], &mut dst[written..], blanks > 0);
        let [r, w] = match cpu.backend() {
            #[cfg(target_arch = "x86_64")]
            // SAFETY: the token names a backend the CPU supports.
            QuotedPrintableBackend::Avx512 => unsafe { avx512::decode(rest, out, prev, dirty) },
            #[cfg(target_arch = "x86_64")]
            // SAFETY: the token names a backend the CPU supports.
            QuotedPrintableBackend::Avx2 => unsafe { avx2::decode(rest, out, prev, dirty) },
            #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
            // SAFETY: the token names a backend the CPU supports.
            QuotedPrintableBackend::Neon => unsafe { neon::decode(rest, out, prev, dirty) },
            QuotedPrintableBackend::Portable => blocks(rest, &end, out, prev, dirty),
        };
        let ran = src[read..read + r].iter().rev().take(BLANKS_MAX + 1);
        let trailing = ran.take_while(|&&b| is_blank(b)).count();
        blanks = if trailing == r { blanks + r } else { trailing };
        read += r;
        written += w;

        match src[read..] {
            [b'=', ref after @ ..] => {
                let spaces = after
                    .iter()
                    .take(BLANKS_MAX + 1)
                    .take_while(|&&byte| is_blank(byte))
                    .count();

                let digit = after
                    .first()
                    .is_some_and(|&byte| DIGITS[1][byte as usize] < 0x8000);
                let eol = match after[spaces..] {
                    _ if spaces > BLANKS_MAX => None,
                    [b'\n', ..] => Some(1),
                    [b'\r', b'\n', ..] => Some(2),
                    [] if last => Some(0),
                    [] | [b'\r'] if !last => return [read, written, 0],
                    [_] if digit && !last => return [read, written, 0],
                    _ => None,
                };

                if let Some(eol) = eol {
                    read += 1 + spaces + eol;
                } else {
                    let kept = 1 + digit as usize;
                    dst[written..written + kept].copy_from_slice(&src[read..read + kept]);
                    *dirty = true;
                    read += kept;
                    written += kept;
                }
            }
            [b'\r', b'\n', ..] | [b'\n', ..] => {
                let len = 1 + (src[read] == b'\r') as usize;
                written -= blanks.min(BLANKS_MAX);
                dst[written..written + len].copy_from_slice(&src[read..read + len]);
                read += len;
                written += len;
            }
            [] | [b'\r'] if !last || read == src.len() => {
                let held = blanks.min(BLANKS_MAX);
                written -= held;
                return [read, written, if last { 0 } else { held }];
            }
            // NOTE: a bare CR, as blocks stop on nothing else. It is data, so
            // the blanks before it stay.
            _ => {
                dst[written] = b'\r';
                *dirty = true;
                read += 1;
                written += 1;
            }
        }

        blanks = 0;
    }
}

/// Decodes Q text `src` into `dst`, at least as long, returning the bytes
/// read, written and held (none).
///
/// A `=` starting no escape stays and sets `dirty`. Unless `last`, a `=`
/// within 2 bytes of the end is left unread.
fn words(src: &[u8], dst: &mut [u8], last: bool, dirty: &mut bool) -> [usize; 3] {
    let mut read = 0;
    let mut written = 0;

    while let Some(&byte) = src.get(read) {
        let (out, len) = match (byte, src.get(read + 1..read + 3)) {
            (b'_', _) => (b' ', 1),
            (b'=', Some(&[high, low])) if escape(high, low) < 0x8000 => {
                (escape(high, low) as u8, 3)
            }
            (b'=', None) if !last => break,
            (b'=', _) => {
                *dirty = true;
                (b'=', 1)
            }
            _ => (byte, 1),
        };

        dst[written] = out;
        read += len;
        written += 1;
    }

    [read, written, 0]
}

/// Decodes `src` into `dst`, at least as long, in 32-byte blocks, up to the
/// first byte left to the token steps, returning the bytes read and written.
///
/// `end` is the input's end as [`pad`] copies it, `prev` tells whether a
/// blank precedes `src`, and `dirty` is set on an unclean byte. A block reads
/// 2 bytes ahead and runs past its end to finish an escape or soft break. A
/// malformed escape, a bare CR or a line break after a blank stops it.
#[inline(never)]
fn blocks(src: &[u8], end: &[u8; 128], dst: &mut [u8], prev: bool, dirty: &mut bool) -> [usize; 2] {
    let mut read = 0;
    let mut written = 0;
    let mut flags = 0;
    let mut spare = [0; 64];

    while src.len() > read {
        let rest = src.len() - read;
        let w = window(&src[read..], end);
        let direct = dst.len() - written >= 64;
        let out = dst[written..].first_chunk_mut().unwrap_or(&mut spare);
        let mut stop = rest.min(32);
        let mut events = !mask(w, is_plain) & u32::MAX >> (32 - stop);
        let mut r = 0;
        let mut off = 64;

        while events != 0 {
            let at = events.trailing_zeros() as usize;
            events &= events - 1;

            if w[at] == b'=' {
                let [high, low] = [w[at + 1], w[at + 2]];
                let value = escape(high, low);
                out[(r + off) & 31..][..32].copy_from_slice(&w[r & 31..][..32]);

                if value < 0x8000 {
                    out[(at + off) & 63] = value as u8;
                    flags |= value;
                    (r, off) = (at + 3, off - 2);
                } else if high == b'\n' {
                    (r, off) = (at + 2, off - 2);
                } else if [high, low] == *b"\r\n" {
                    (r, off) = (at + 3, off - 3);
                } else {
                    (r, stop) = (at, at);
                    break;
                }
            } else if at < r {
            } else if matches!(w[at], b'\r' | b'\n') {
                let blank = (read + at)
                    .checked_sub(1)
                    .map_or(prev, |i| is_blank(src[i]));

                if blank || w[at] == b'\r' && w[at + 1] != b'\n' {
                    stop = at;
                    break;
                }
            } else {
                flags |= 0x1000;
            }
        }

        if r < stop {
            out[(r + off) & 31..][..32].copy_from_slice(&w[r & 31..][..32]);
            r = stop;
        }

        let n = r + off - 64;

        if !direct {
            dst[written..written + n].copy_from_slice(&spare[..n]);
        }

        read += r;
        written += n;

        if stop < rest.min(32) {
            break;
        }
    }

    *dirty |= flags & 0x1000 != 0;
    [read, written]
}

/// The byte the escape `=` `high` `low` stands for, 0x1000 added when a
/// digit is lowercase, or 0x8000 or more when either is no hex digit.
fn escape(high: u8, low: u8) -> u16 {
    DIGITS[0][high as usize] | DIGITS[1][low as usize]
}

/// The value of each byte as the high and the low hex digit of an escape, as
/// [`escape`] returns it.
static DIGITS: [[u16; 256]; 2] = {
    let mut table = [[0x8000; 256]; 2];
    let mut digit = 0;

    while digit < 22 {
        let byte = b"0123456789ABCDEFabcdef"[digit] as usize;
        let [value, flag] = [
            digit as u16 % 16 + digit as u16 / 16 * 10,
            (digit as u16 / 16) << 12,
        ];
        table[0][byte] = value << 4 | flag;
        table[1][byte] = value | flag;
        digit += 1;
    }

    table
};

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};

    use simd_cpu::SimdCpu;

    use crate::{
        codec::{QuotedPrintableEol, QuotedPrintableMode},
        cpu::{QuotedPrintableCpu, available},
        decode::*,
    };

    const CODEC: QuotedPrintableCodec = QuotedPrintableCodec {
        cpu: QuotedPrintableCpu::new(SimdCpu::BASELINE),
        eol: QuotedPrintableEol::Crlf,
        mode: QuotedPrintableMode::Text,
        guard: false,
    };

    const Q: QuotedPrintableCodec = QuotedPrintableCodec {
        mode: QuotedPrintableMode::Q,
        ..CODEC
    };

    /// The decoding of `input` by `codec` under every backend, checked the
    /// same.
    fn decoded(codec: QuotedPrintableCodec, input: &[u8]) -> (Vec<u8>, bool) {
        let outs: Vec<_> = available()
            .into_iter()
            .map(|cpu| {
                let codec = QuotedPrintableCodec { cpu, ..codec };
                let mut out = vec![0; codec.decoded_len_max(input.len())];
                let output = codec.decode(input, &mut out).unwrap();
                out.truncate(output.written);
                (out, output.clean)
            })
            .collect();

        assert!(outs.windows(2).all(|pair| pair[0] == pair[1]));
        outs[0].clone()
    }

    fn clean(bytes: &[u8]) -> (Vec<u8>, bool) {
        (bytes.to_vec(), true)
    }

    fn unclean(bytes: &[u8]) -> (Vec<u8>, bool) {
        (bytes.to_vec(), false)
    }

    #[test]
    fn escapes_become_bytes() {
        assert_eq!(decoded(CODEC, b"a=3Db=C3=A9"), clean("a=bé".as_bytes()));
        assert_eq!(decoded(CODEC, b"=c3=a9"), unclean("é".as_bytes()));
    }

    #[test]
    fn soft_breaks_are_joined() {
        assert_eq!(
            decoded(CODEC, b"ab=\r\ncd=\nef= \t\r\ng="),
            clean(b"abcdefg")
        );
        assert_eq!(decoded(CODEC, b"ab= "), clean(b"ab"));
    }

    #[test]
    fn line_final_blanks_are_deleted() {
        assert_eq!(decoded(CODEC, b"a \t\r\nb \nc  "), clean(b"a\r\nb\nc"));
        assert_eq!(decoded(CODEC, b"a  b"), clean(b"a  b"));
    }

    #[test]
    fn a_bare_cr_is_kept_and_reported() {
        assert_eq!(decoded(CODEC, b"a\rb\r"), unclean(b"a\rb\r"));
        assert_eq!(decoded(CODEC, b"a \r"), unclean(b"a \r"));
        assert_eq!(decoded(CODEC, b"a \r\r\nb"), unclean(b"a \r\r\nb"));
        assert_eq!(decoded(CODEC, b"a\r \n"), unclean(b"a\r\n"));
    }

    #[test]
    fn malformed_escapes_are_kept_and_reported() {
        assert_eq!(decoded(CODEC, b"a=zb"), unclean(b"a=zb"));
        assert_eq!(decoded(CODEC, b"a=4zb"), unclean(b"a=4zb"));
        assert_eq!(decoded(CODEC, b"a=4"), unclean(b"a=4"));
        assert_eq!(decoded(CODEC, b"a= b"), unclean(b"a= b"));
        assert_eq!(decoded(CODEC, b"a= \rb"), unclean(b"a= \rb"));
        assert_eq!(decoded(CODEC, b"a==41"), unclean(b"a=A"));
    }

    #[test]
    fn unsafe_bytes_are_kept_and_reported() {
        assert_eq!(decoded(CODEC, b"a\x00b"), unclean(b"a\x00b"));
        assert_eq!(decoded(CODEC, "é".as_bytes()), unclean("é".as_bytes()));
    }

    #[test]
    fn too_many_blanks_are_kept() {
        let mut input = vec![b' '; BLANKS_MAX + 2];
        input.extend_from_slice(b"\r\n");
        let mut expected = vec![b' '; 2];
        expected.extend_from_slice(b"\r\n");
        assert_eq!(decoded(CODEC, &input), clean(&expected));
    }

    #[test]
    fn decoding_reverses_encoding() {
        let input: Vec<u8> = (0..=255)
            .chain(b"a \r\nb \rc\t\n".iter().copied())
            .cycle()
            .take(2000)
            .collect();

        for mode in [
            QuotedPrintableMode::Text,
            QuotedPrintableMode::Binary,
            QuotedPrintableMode::Q,
        ] {
            let codec = QuotedPrintableCodec { mode, ..CODEC };
            let mut encoded = vec![0; codec.encoded_len_max(input.len()).unwrap()];
            let len = codec.encode(&input, &mut encoded).unwrap();
            let (out, is_clean) = decoded(codec, &encoded[..len]);
            assert!(is_clean);

            if mode == QuotedPrintableMode::Text {
                assert!(
                    encoded[..len]
                        .split(|&b| b == b'\n')
                        .all(|line| line.len() <= 77)
                );
            } else {
                assert_eq!(out, input);
            }
        }
    }

    #[test]
    fn q_reads_underscores_as_spaces() {
        assert_eq!(
            decoded(Q, b"caf=C3=A9_au_lait"),
            clean("café au lait".as_bytes())
        );
        assert_eq!(decoded(Q, b"caf=c3=a9"), clean("café".as_bytes()));
    }

    #[test]
    fn q_reports_only_a_broken_escape() {
        assert_eq!(decoded(Q, b"a=zb"), unclean(b"a=zb"));
        assert_eq!(decoded(Q, b"a=4"), unclean(b"a=4"));
        assert_eq!(decoded(Q, b"a="), unclean(b"a="));
        assert_eq!(decoded(Q, b"a==41"), unclean(b"a=A"));
        assert_eq!(decoded(Q, b"ab=\r\ncd"), unclean(b"ab=\r\ncd"));
        assert_eq!(decoded(Q, b"x \t\r\n\r\xe9?"), clean(b"x \t\r\n\r\xe9?"));
    }

    #[test]
    fn one_byte_of_room_always_progresses() {
        let mut input = vec![b'\t'; BLANKS_MAX];
        input.extend_from_slice(b"x=41=\r\n");

        for codec in available()
            .iter()
            .flat_map(|&cpu| [CODEC, Q].map(|codec| QuotedPrintableCodec { cpu, ..codec }))
        {
            let mut decoder = QuotedPrintableDecode::new(codec);
            let mut src = &input[..];
            let mut out = Vec::new();
            let mut room = [0; 1];

            while !src.is_empty() {
                let output = decoder.resume(src, &mut room);
                assert!(output.read > 0 || output.written > 0);
                out.extend_from_slice(&room[..output.written]);
                src = &src[output.read..];
            }

            // NOTE: in Q mode the input ends in bytes still queued.
            while decoder.resume(&[], &mut room).written > 0 {
                out.push(room[0]);
            }

            assert_eq!(decoder.finish(&mut room).unwrap().written, 0);
            assert_eq!(out, decoded(codec, &input).0);
        }
    }

    #[test]
    fn a_short_buffer_writes_nothing() {
        let mut out = [0xaa; 2];
        assert_eq!(CODEC.decode(b"abc", &mut out), None);
        assert_eq!(out, [0xaa; 2]);

        for cpu in available() {
            let mut decoder = QuotedPrintableDecode::new(QuotedPrintableCodec { cpu, ..CODEC });
            decoder.resume(b"=4", &mut []);
            assert_eq!(decoder.finish(&mut out[..1]), None);
            assert_eq!(decoder.finish(&mut out).map(|end| end.written), Some(2));
            assert_eq!(&out, b"=4");
        }
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn a_vec_is_appended_to() {
        let mut out = b"> ".to_vec();
        assert!(CODEC.decode_vec(b"a=3Db", &mut out));
        assert_eq!(out, b"> a=b");
    }
}
