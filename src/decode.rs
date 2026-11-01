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
//! [`QuotedPrintableDecode`] takes the input chunk by chunk; [`decode`] and
//! `decode_vec` run its core once over a whole input. Encoding is the
//! sibling module [`encode`](crate::encode).

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::{edge::Edge, is_blank, is_plain, mask, window};

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
/// use quoted_printable_codec::decode::QuotedPrintableDecode;
///
/// let mut decoder = QuotedPrintableDecode::new();
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
#[derive(Clone, Debug)]
pub struct QuotedPrintableDecode {
    edge: Edge<CARRY, CARRY>,
    clean: bool,
}

impl Default for QuotedPrintableDecode {
    fn default() -> Self {
        Self::new()
    }
}

impl QuotedPrintableDecode {
    /// Builds a decoder.
    pub fn new() -> Self {
        Self {
            edge: Edge::new(),
            clean: true,
        }
    }

    /// Decodes as much of `src` as fits in `dst`.
    ///
    /// The caller drains `dst` and resumes with the rest of `src` or the
    /// next chunk; one byte of room always lets the decoder progress. The
    /// bytes of `dst` past the ones written may be overwritten.
    pub fn resume(&mut self, src: &[u8], dst: &mut [u8]) -> QuotedPrintableDecodeOutput {
        let clean = &mut self.clean;
        let (read, written) = self.edge.resume(src, dst, |src, dst, last| {
            tokens(&src[..src.len().min(dst.len())], dst, last, clean)
        });

        QuotedPrintableDecodeOutput {
            read,
            written,
            clean: self.clean,
        }
    }

    /// Ends the input: writes what is still queued and a malformed escape
    /// left unfinished, and drops the trailing spaces, tabs and CR.
    ///
    /// Returns none, writing nothing, when `dst` is too short.
    pub fn finish(&mut self, dst: &mut [u8]) -> Option<QuotedPrintableDecodeOutput> {
        let mut clean = self.clean;
        let written = self
            .edge
            .finish(dst, |src, dst| tokens(src, dst, true, &mut clean)[1])?;
        self.clean = clean;

        Some(QuotedPrintableDecodeOutput {
            read: 0,
            written,
            clean,
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

/// Decodes the whole of `src` into `dst`.
///
/// Returns none, writing nothing, when `dst` is shorter than
/// [`decoded_len_max`] asks. Within that length, the bytes of `dst` past the
/// ones written may be overwritten.
pub fn decode(src: &[u8], dst: &mut [u8]) -> Option<QuotedPrintableDecodeOutput> {
    let dst = dst.get_mut(..decoded_len_max(src.len()))?;
    let mut clean = true;
    let [read, written, _] = tokens(src, dst, true, &mut clean);

    Some(QuotedPrintableDecodeOutput {
        read,
        written,
        clean,
    })
}

/// Decodes the whole of `src` at the end of `out`, returning whether it was
/// well-formed.
#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
pub fn decode_vec(src: &[u8], out: &mut Vec<u8>) -> bool {
    let start = out.len();
    out.resize(start + decoded_len_max(src.len()), 0);

    let mut clean = true;
    let [_, written, _] = tokens(src, &mut out[start..], true, &mut clean);
    out.truncate(start + written);
    clean
}

/// The most bytes [`decode`] writes for an input of `len` bytes, whatever
/// they hold: every output byte comes from at least one input byte.
pub const fn decoded_len_max(len: usize) -> usize {
    len
}

/// Decodes the tokens of `src` into `dst`, at least as long, returning the
/// bytes read, written and held.
///
/// Unless `last` tells that the input ends with `src`, an escape, a soft
/// break or a CR cut short is left unread, and the spaces and tabs a line
/// break would delete are held: written past the ones counted, for the
/// caller to carry.
fn tokens(src: &[u8], dst: &mut [u8], last: bool, clean: &mut bool) -> [usize; 3] {
    let mut read = 0;
    let mut written = 0;
    let mut blanks = 0;
    let mut irregular = 0;

    loop {
        if read >= irregular && read < src.len() {
            match block(&src[read..], &mut dst[written..], blanks > 0, clean) {
                Ok([r, w, trailing]) => {
                    read += r;
                    written += w;
                    blanks = if trailing == r { blanks + r } else { trailing };
                    continue;
                }
                Err(at) => irregular = read + at + 1,
            }
        }

        match src[read..] {
            [b'=', high, low, ..] if (HEX[high as usize] | HEX[low as usize]) < 0x80 => {
                let (high, low) = (HEX[high as usize], HEX[low as usize]);
                *clean &= (high | low) & 0x10 == 0;
                dst[written] = high << 4 | low & 0xf;
                read += 3;
                written += 1;
            }
            [b'=', ref after @ ..] => {
                let spaces = after
                    .iter()
                    .take(BLANKS_MAX + 1)
                    .take_while(|&&byte| is_blank(byte))
                    .count();

                let digit = after.first().is_some_and(|&byte| HEX[byte as usize] < 0x80);
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
                    *clean = false;
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
            [b'\r', ref rest @ ..] if last || !rest.is_empty() => {
                *clean = false;
                read += 1;
                continue;
            }
            [] | [b'\r'] => {
                let held = blanks.min(BLANKS_MAX);
                written -= held;
                return [read, written, if last { 0 } else { held }];
            }
            [byte, ..] => {
                dst[written] = byte;
                *clean &= is_plain(byte);
                blanks = if is_blank(byte) { blanks + 1 } else { 0 };
                read += 1;
                written += 1;
                continue;
            }
        }

        blanks = 0;
    }
}

/// Decodes the first 32 bytes of `src`, or all of them when fewer, into
/// `dst`, returning the bytes read, written, and the spaces and tabs that
/// end them; `blank` tells that a space or tab the next line break would
/// delete comes before them.
///
/// The 2 bytes after them are read ahead, and the bytes read run past them
/// to the end of an escape or a soft break that starts in them. They hold
/// only literal bytes, escapes, soft breaks and CRLF or LF with no blank
/// before them, or the offset of the first other one is returned, and the
/// token steps decode them instead.
// NOTE: folded into `tokens`, its loop runs out of registers: 70% slower on
// escapes.
fn block(src: &[u8], dst: &mut [u8], blank: bool, clean: &mut bool) -> Result<[usize; 3], usize> {
    let mut buffer = [0; 64];
    let w = window(src, &mut buffer);
    let len = src.len().min(32);
    let lanes = ((1u64 << len) - 1) as u32;
    let mut equals = mask(w, |byte| byte == b'=') & lanes;
    let mut others = !mask(w, is_plain) & !equals & lanes;

    let mut spare = [0; 64];
    let direct = dst.len() >= 64;
    let out = if direct {
        &mut dst[..64]
    } else {
        &mut spare[..]
    };
    let mut read = 0;
    let mut written = 0;
    let mut dirty = 0;

    while equals != 0 {
        let at = equals.trailing_zeros() as usize;
        let [high, low] = [w[at + 1], w[at + 2]];
        let digits = HEX[high as usize] | HEX[low as usize];
        let escape = digits < 0x80;

        if !escape && high != b'\n' && [high, low] != *b"\r\n" {
            return Err(at);
        }

        out[written & 31..][..32].copy_from_slice(&w[read & 31..][..32]);
        written += at - read;
        out[written & 63] = HEX[high as usize] << 4 | HEX[low as usize] & 0xf;
        written += escape as usize;
        dirty |= digits & (escape as u8) << 4;
        read = at + 2 + (escape | (high == b'\r')) as usize;
        equals &= equals - 1;
    }

    while others != 0 {
        let at = others.trailing_zeros() as usize;
        let before = if at > 0 {
            w[at - 1]
        } else {
            blank as u8 * b' '
        };
        let soft = before == b'=' || at > 1 && w[at - 2..at] == *b"=\r";

        match w[at] {
            b'\r' | b'\n' if soft => (),
            b'\r' if w[at + 1] != b'\n' => return Err(at),
            b'\r' | b'\n' if is_blank(before) => return Err(at),
            b'\r' | b'\n' => (),
            _ => dirty |= 1,
        }

        others &= others - 1;
    }

    if read < len {
        out[written & 31..][..32].copy_from_slice(&w[read..][..32]);
        written += len - read;
        read = len;
    }

    if !direct {
        let len = dst.len();
        dst.copy_from_slice(&spare[..len]);
    }

    *clean &= dirty == 0;
    let trailing = w[..len]
        .iter()
        .rev()
        .take_while(|&&byte| is_blank(byte))
        .count();
    Ok([read, written, trailing])
}

/// The value of each byte as a hex digit, 0x10 added to a lowercase one, or
/// 0x80 for any other byte.
static HEX: [u8; 256] = {
    let mut table = [0x80; 256];
    let mut digit = 0;

    while digit < 16 {
        table[b"0123456789ABCDEF"[digit] as usize] = digit as u8;
        table[b"0123456789abcdef"[digit] as usize] = digit as u8 | (digit as u8 / 10) << 4;
        digit += 1;
    }

    table
};

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};

    use crate::{
        decode::*,
        encode::{QuotedPrintableEncodeOptions, encode, encoded_len_max},
    };

    fn decoded(input: &[u8]) -> (Vec<u8>, bool) {
        let mut out = vec![0; decoded_len_max(input.len())];
        let output = decode(input, &mut out).unwrap();
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
            let opts = QuotedPrintableEncodeOptions {
                binary,
                ..Default::default()
            };

            let mut encoded = vec![0; encoded_len_max(input.len(), opts).unwrap()];
            let len = encode(&input, &mut encoded, opts).unwrap();
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

        let mut decoder = QuotedPrintableDecode::new();
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
        assert_eq!(decode(b"abc", &mut out), None);
        assert_eq!(out, [0xaa; 2]);

        let mut decoder = QuotedPrintableDecode::new();
        decoder.resume(b"=4", &mut []);
        assert_eq!(decoder.finish(&mut out[..1]), None);
        assert_eq!(decoder.finish(&mut out).map(|end| end.written), Some(2));
        assert_eq!(&out, b"=4");
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn a_vec_is_appended_to() {
        let mut out = b"> ".to_vec();
        assert!(decode_vec(b"a=3Db", &mut out));
        assert_eq!(out, b"> a=b");
    }
}
