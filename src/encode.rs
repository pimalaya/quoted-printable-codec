//! # Encoding
//!
//! Quoted-printable encoding of a MIME body, in lines of at most 76
//! characters soft break included (RFC 2045 6.7).
//!
//! In text mode, LF and CRLF become hard line breaks; in binary mode, CR and
//! LF are escaped like any other byte and the output has soft line breaks
//! only. `=`, bytes from 0x7F up, control characters other than TAB, a bare
//! CR in text mode and a space or tab that would end a line are escaped as
//! `=XX`, uppercase; an escape is never split across lines.
//!
//! [`QuotedPrintableEncode`] takes the input chunk by chunk; the one-shot
//! [`QuotedPrintableCodec::encode`] and `QuotedPrintableCodec::encode_vec`
//! encode a whole input. Decoding is the sibling module
//! [`decode`](crate::decode).

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::{codec::QuotedPrintableCodec, edge::Edge, is_blank, is_plain, mask, pad, window};

/// The most characters a line holds before its soft break `=`.
const LINE: usize = 75;

/// The most output bytes a block touches: 32 escapes, two soft breaks, and a
/// run copied whole past them.
const ROOM: usize = 3 * 32 + 6 + 32;

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
    /// Builds an encoder running the mode and the line ending of `codec`.
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
    /// Returns the bytes written, or none, writing nothing, when `dst` is
    /// too short.
    pub fn finish(&mut self, dst: &mut [u8]) -> Option<usize> {
        let mut column = self.column;
        self.edge.finish(dst, |src, dst| {
            blocks(src, dst, true, self.codec, &mut column)[1]
        })
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
        let mut column = 0;
        let [read, written, _] = blocks(src, dst, true, self, &mut column);

        if read == src.len() {
            return Some(written);
        }

        let mut spare = [0; 2 * ROOM];
        let [_, rest, _] = blocks(&src[read..], &mut spare, true, self, &mut column);
        dst[written..written + rest].copy_from_slice(&spare[..rest]);
        Some(written + rest)
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

    /// The most bytes [`encode`](Self::encode) writes for an input of `len`
    /// bytes, whatever they hold, or none when that overflows `usize`.
    ///
    /// Every byte takes at most three characters, and a soft break follows
    /// at least 73 characters, so at least 25 input bytes.
    pub fn encoded_len_max(self, len: usize) -> Option<usize> {
        let breaks = (len / 25 + 1).checked_mul(self.eol.soft_break().1)?;
        len.checked_mul(3)?.checked_add(breaks)
    }
}

/// Encodes `src` into `dst` from `column` in blocks of 32 bytes, as many as
/// fit, returning the bytes read and written, and none held, as [`Edge`]
/// asks.
///
/// Each block reads the 2 bytes after it ahead. Unless `last` tells that the
/// input ends with `src`, a space, tab or CR that may end a line is left
/// unread when `src` ends before telling.
fn blocks(
    src: &[u8],
    dst: &mut [u8],
    last: bool,
    codec: QuotedPrintableCodec,
    column: &mut usize,
) -> [usize; 3] {
    let text = !codec.binary;
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
        let mut events = !mask(w, is_plain) & u32::MAX >> (32 - len);

        if end && is_blank(w[len - 1]) {
            events |= 1 << (len - 1);
        }

        let mut from = 0;
        let mut n = 0;

        loop {
            let at = if events == 0 {
                len
            } else {
                events.trailing_zeros() as usize
            };
            let line = at < len && breaks(at);

            if line && at > from && is_blank(w[at - 1]) {
                events |= 1 << (at - 1);
                continue;
            }

            let mut run = at - from;

            if col + run > LINE {
                let first = LINE - col;
                out[n..][..32].copy_from_slice(&w[from..][..32]);
                out[n + first..][..4].copy_from_slice(&soft);
                n += first + soft_len;
                from += first;
                run -= first;
                col = 0;
            }

            out[n..][..32].copy_from_slice(&w[from..][..32]);
            n += run;
            col += run;

            if at == len {
                break;
            }

            if !line {
                if col > LINE - 3 {
                    out[n..][..4].copy_from_slice(&soft);
                    n += soft_len;
                    col = 0;
                }

                out[n..][..4].copy_from_slice(&ESCAPES[w[at] as usize]);
                n += 3;
                col += 3;
            } else if w[at] == b'\n' {
                out[n..][..3].copy_from_slice(&soft[1..]);
                n += soft_len - 1;
                col = 0;
            }

            from = at + 1;
            events &= events - 1;
        }

        read += len;
        written += n;
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

    use crate::{codec::QuotedPrintableEol, encode::*};

    const TEXT: QuotedPrintableCodec = QuotedPrintableCodec {
        eol: QuotedPrintableEol::Crlf,
        binary: false,
    };

    const BINARY: QuotedPrintableCodec = QuotedPrintableCodec {
        binary: true,
        ..TEXT
    };

    fn encoded(input: &[u8], codec: QuotedPrintableCodec) -> Vec<u8> {
        let mut out = vec![0; codec.encoded_len_max(input.len()).unwrap()];
        let written = codec.encode(input, &mut out).unwrap();
        out.truncate(written);
        out
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
                for codec in [TEXT, BINARY] {
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

        for codec in [TEXT, BINARY] {
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

    #[test]
    fn a_short_buffer_writes_nothing() {
        let mut out = [0xaa; 3];
        assert_eq!(TEXT.encode(b"f", &mut out), None);
        assert_eq!(out, [0xaa; 3]);

        let mut encoder = QuotedPrintableEncode::new(TEXT);
        assert_eq!(encoder.resume(b" ", &mut []).read, 1);
        assert_eq!(encoder.finish(&mut out[..2]), None);
        assert_eq!(encoder.finish(&mut out), Some(3));
        assert_eq!(&out, b"=20");
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn a_vec_is_appended_to() {
        let mut out = b"> ".to_vec();
        TEXT.encode_vec(b"a=b", &mut out);
        assert_eq!(out, b"> a=3Db");
    }
}
