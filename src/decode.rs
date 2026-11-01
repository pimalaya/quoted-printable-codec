//! # Decoding
//!
//! Lenient quoted-printable decoding of a MIME body, as RFC 2045 6.7 asks
//! of robust implementations: `=XX` escapes become bytes, lowercase hex
//! digits included; soft breaks (`=`, optional spaces or tabs, then a line
//! break or the end of the input) are joined; the spaces and tabs that end a
//! line or the input are deleted; CRLF and bare LF are kept as they are, and
//! a bare CR is dropped; a malformed escape keeps its `=` literally, and
//! anything else is kept. Decoding never fails: it returns the bytes, and
//! whether the input was clean.
//!
//! [`QuotedPrintableDecode`] takes the input chunk by chunk; the one-shot
//! [`QuotedPrintableCodec::decode`] and `QuotedPrintableCodec::decode_vec`
//! run its core once over a whole input. Encoding is the sibling module
//! [`encode`](crate::encode).

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::{codec::QuotedPrintableCodec, edge::Edge, is_blank, is_plain, mask, pad, window};

/// The most trailing spaces and tabs a line break deletes: past them, the
/// oldest are kept as data, as no well-formed line holds that many.
const BLANKS_MAX: usize = 76;

/// The longest token the decoder cannot decide yet, `=`, the most blanks
/// and a CR, and room for one more byte.
const CARRY: usize = BLANKS_MAX + 4;

/// I/O-free quoted-printable decoder, resumed with the caller's input and
/// output buffers.
///
/// The input can be cut anywhere, mid-escape included: the decoder carries
/// an unfinished escape or soft break, a CR, and the spaces and tabs that
/// may end a line.
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
    edge: Edge<CARRY, CARRY>,
    dirty: bool,
}

impl QuotedPrintableDecode {
    /// Builds a decoder, whatever the mode and the line ending of the codec:
    /// decoding takes any.
    pub fn new(_: QuotedPrintableCodec) -> Self {
        Self::default()
    }

    /// Decodes as much of `src` as fits in `dst`.
    ///
    /// The caller drains `dst` and resumes with the rest of `src` or the
    /// next chunk; one byte of room always lets the decoder progress. The
    /// bytes of `dst` past the ones written may be overwritten.
    pub fn resume(&mut self, src: &[u8], dst: &mut [u8]) -> QuotedPrintableDecodeOutput {
        let dirty = &mut self.dirty;
        let (read, written) = self.edge.resume(src, dst, |src, dst, last| {
            tokens(&src[..src.len().min(dst.len())], dst, last, dirty)
        });

        QuotedPrintableDecodeOutput {
            read,
            written,
            clean: !self.dirty,
        }
    }

    /// Ends the input: writes what is still queued and a malformed escape
    /// left unfinished, and drops the trailing spaces, tabs and CR.
    ///
    /// Returns none, writing nothing, when `dst` is too short; otherwise the
    /// decoder starts over for another input.
    pub fn finish(&mut self, dst: &mut [u8]) -> Option<QuotedPrintableDecodeOutput> {
        let mut dirty = self.dirty;
        let written = self
            .edge
            .finish(dst, |src, dst| tokens(src, dst, true, &mut dirty)[1])?;
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
    /// [`decoded_len_max`](Self::decoded_len_max) asks. The bytes of `dst`
    /// past the ones written may be overwritten, to its end.
    #[inline]
    pub fn decode(self, src: &[u8], dst: &mut [u8]) -> Option<QuotedPrintableDecodeOutput> {
        if dst.len() < self.decoded_len_max(src.len()) {
            return None;
        }

        let mut dirty = false;
        let [read, written, _] = tokens(src, dst, true, &mut dirty);

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

    /// The most bytes [`decode`](Self::decode) writes for an input of `len`
    /// bytes, whatever they hold: every output byte comes from at least one
    /// input byte.
    pub const fn decoded_len_max(self, len: usize) -> usize {
        len
    }
}

/// Decodes the tokens of `src` into `dst`, at least as long, returning the
/// bytes read, written and held.
///
/// Unless `last` tells that the input ends with `src`, an escape, a soft
/// break or a CR cut short is left unread, and the spaces and tabs a line
/// break would delete are held: written past the ones counted, for the
/// caller to carry.
fn tokens(src: &[u8], dst: &mut [u8], last: bool, dirty: &mut bool) -> [usize; 3] {
    let mut read = 0;
    let mut written = 0;
    let mut blanks = 0;
    // NOTE: padded first, so that its stores are done when blocks load them.
    let mut end = [0; 128];
    pad(&mut end, src);

    loop {
        let [r, w] = blocks(&src[read..], &end, &mut dst[written..], &mut blanks, dirty);
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
            // NOTE: a bare CR, as blocks stop on nothing else.
            _ => {
                *dirty = true;
                read += 1;
                continue;
            }
        }

        blanks = 0;
    }
}

/// Decodes `src` into `dst`, at least as long, in blocks of 32 bytes, up to
/// the first byte a block leaves to the token steps or the end of `src`,
/// returning the bytes read and written; `end` holds the end of the input as
/// [`pad`] copies it, `blanks` counts the spaces and tabs the next line break
/// would delete, and `dirty` turns true on a byte that is not clean.
///
/// A block reads the 2 bytes after it ahead, and runs past its end to the end
/// of an escape or a soft break that starts in it. It decodes literal bytes,
/// escapes, soft breaks, and CRLF or LF with no blank before them: a
/// malformed escape, a bare CR or a line break after a blank ends it.
#[inline(never)]
fn blocks(
    src: &[u8],
    end: &[u8; 128],
    dst: &mut [u8],
    blanks: &mut usize,
    dirty: &mut bool,
) -> [usize; 2] {
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
                let blank = at.checked_sub(1).map_or(*blanks > 0, |at| is_blank(w[at]));

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

        let trailing = w[..r]
            .iter()
            .rev()
            .take_while(|&&byte| is_blank(byte))
            .count();
        *blanks = if trailing == r { *blanks + r } else { trailing };
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

    use crate::{codec::QuotedPrintableEol, decode::*};

    const CODEC: QuotedPrintableCodec = QuotedPrintableCodec {
        eol: QuotedPrintableEol::Crlf,
        binary: false,
    };

    fn decoded(input: &[u8]) -> (Vec<u8>, bool) {
        let mut out = vec![0; CODEC.decoded_len_max(input.len())];
        let output = CODEC.decode(input, &mut out).unwrap();
        out.truncate(output.written);
        (out, output.clean)
    }

    fn clean(bytes: &[u8]) -> (Vec<u8>, bool) {
        (bytes.to_vec(), true)
    }

    fn unclean(bytes: &[u8]) -> (Vec<u8>, bool) {
        (bytes.to_vec(), false)
    }

    #[test]
    fn escapes_become_bytes() {
        assert_eq!(decoded(b"a=3Db=C3=A9"), clean("a=bé".as_bytes()));
        assert_eq!(decoded(b"=c3=a9"), unclean("é".as_bytes()));
    }

    #[test]
    fn soft_breaks_are_joined() {
        assert_eq!(decoded(b"ab=\r\ncd=\nef= \t\r\ng="), clean(b"abcdefg"));
        assert_eq!(decoded(b"ab= "), clean(b"ab"));
    }

    #[test]
    fn line_final_blanks_are_deleted() {
        assert_eq!(decoded(b"a \t\r\nb \nc  "), clean(b"a\r\nb\nc"));
        assert_eq!(decoded(b"a  b"), clean(b"a  b"));
    }

    #[test]
    fn a_bare_cr_is_dropped_and_reported() {
        assert_eq!(decoded(b"a\rb"), unclean(b"ab"));
        assert_eq!(decoded(b"a \r"), unclean(b"a"));
    }

    #[test]
    fn malformed_escapes_are_kept_and_reported() {
        assert_eq!(decoded(b"a=zb"), unclean(b"a=zb"));
        assert_eq!(decoded(b"a=4zb"), unclean(b"a=4zb"));
        assert_eq!(decoded(b"a=4"), unclean(b"a=4"));
        assert_eq!(decoded(b"a= b"), unclean(b"a= b"));
        assert_eq!(decoded(b"a= \rb"), unclean(b"a= b"));
        assert_eq!(decoded(b"a==41"), unclean(b"a=A"));
    }

    #[test]
    fn unsafe_bytes_are_kept_and_reported() {
        assert_eq!(decoded(b"a\x00b"), unclean(b"a\x00b"));
        assert_eq!(decoded("é".as_bytes()), unclean("é".as_bytes()));
    }

    #[test]
    fn too_many_blanks_are_kept() {
        let mut input = vec![b' '; BLANKS_MAX + 2];
        input.extend_from_slice(b"\r\n");
        let mut expected = vec![b' '; 2];
        expected.extend_from_slice(b"\r\n");
        assert_eq!(decoded(&input), clean(&expected));
    }

    #[test]
    fn decoding_reverses_encoding() {
        let input: Vec<u8> = (0..=255)
            .chain(b"a \r\nb \rc\t\n".iter().copied())
            .cycle()
            .take(2000)
            .collect();

        for binary in [false, true] {
            let codec = QuotedPrintableCodec { binary, ..CODEC };
            let mut encoded = vec![0; codec.encoded_len_max(input.len()).unwrap()];
            let len = codec.encode(&input, &mut encoded).unwrap();
            let (out, is_clean) = decoded(&encoded[..len]);
            assert!(is_clean);

            if binary {
                assert_eq!(out, input);
            } else {
                assert!(
                    encoded[..len]
                        .split(|&b| b == b'\n')
                        .all(|line| line.len() <= 77)
                );
            }
        }
    }

    #[test]
    fn one_byte_of_room_always_progresses() {
        let mut input = vec![b'\t'; BLANKS_MAX];
        input.extend_from_slice(b"x=41=\r\n");

        let mut decoder = QuotedPrintableDecode::new(CODEC);
        let mut src = &input[..];
        let mut out = Vec::new();
        let mut room = [0; 1];

        while !src.is_empty() {
            let output = decoder.resume(src, &mut room);
            assert!(output.read > 0 || output.written > 0);
            out.extend_from_slice(&room[..output.written]);
            src = &src[output.read..];
        }

        assert_eq!(decoder.finish(&mut room).unwrap().written, 0);
        assert_eq!(out, decoded(&input).0);
    }

    #[test]
    fn a_short_buffer_writes_nothing() {
        let mut out = [0xaa; 2];
        assert_eq!(CODEC.decode(b"abc", &mut out), None);
        assert_eq!(out, [0xaa; 2]);

        let mut decoder = QuotedPrintableDecode::new(CODEC);
        decoder.resume(b"=4", &mut []);
        assert_eq!(decoder.finish(&mut out[..1]), None);
        assert_eq!(decoder.finish(&mut out).map(|end| end.written), Some(2));
        assert_eq!(&out, b"=4");
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn a_vec_is_appended_to() {
        let mut out = b"> ".to_vec();
        assert!(CODEC.decode_vec(b"a=3Db", &mut out));
        assert_eq!(out, b"> a=b");
    }
}
