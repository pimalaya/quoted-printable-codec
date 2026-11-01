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
//! [`QuotedPrintableDecode`] takes the input chunk by chunk; [`decode`] and `decode_vec`
//! run it once over a whole input. Encoding is the sibling module
//! [`encode`](crate::encode).

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::queue::Queue;

/// The most trailing spaces and tabs the decoder carries: past them, the
/// oldest are kept as data, as no well-formed line holds that many.
const BLANKS_MAX: usize = 76;

/// The longest output of one input byte: the carried blanks, a kept `=` and
/// its hex digit, and the byte itself.
const STEP_MAX: usize = BLANKS_MAX + 3;

/// I/O-free quoted-printable decoder, resumed with the caller's input and
/// output buffers.
///
/// The input can be cut anywhere, mid-escape included: the decoder carries
/// an unfinished escape, a CR and the spaces and tabs that may end a line.
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
    state: State,
    blanks: [u8; BLANKS_MAX],
    blanks_len: usize,
    clean: bool,
    queue: Queue<STEP_MAX>,
}

/// Where the decoder stands in the input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
    /// In data, the carried blanks possibly ending the line.
    Text,
    /// After a CR in data.
    Cr,
    /// After `=`.
    Equals,
    /// After `=` and a hex digit.
    Hex(u8),
    /// After `=` and spaces or tabs, carried as blanks.
    EqualsBlanks,
    /// After `=`, spaces or tabs and a CR.
    EqualsCr,
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
            state: State::Text,
            blanks: [0; BLANKS_MAX],
            blanks_len: 0,
            clean: true,
            queue: Queue::new(),
        }
    }

    /// Decodes as much of `src` as fits in `dst`.
    ///
    /// The caller drains `dst` and resumes with the rest of `src` or the
    /// next chunk; one byte of room always lets the decoder progress.
    pub fn resume(&mut self, src: &[u8], dst: &mut [u8]) -> QuotedPrintableDecodeOutput {
        let mut read = 0;
        let mut written = 0;

        loop {
            written += self.queue.drain(&mut dst[written..]);

            if !self.queue.is_empty() || read == src.len() {
                return QuotedPrintableDecodeOutput {
                    read,
                    written,
                    clean: self.clean,
                };
            }

            self.step(src[read]);
            read += 1;
        }
    }

    /// Ends the input: writes what is still queued and a malformed escape
    /// left unfinished, and drops the trailing spaces, tabs and CR.
    ///
    /// Returns none, writing nothing, when `dst` is too short.
    pub fn finish(&mut self, dst: &mut [u8]) -> Option<QuotedPrintableDecodeOutput> {
        match self.state {
            State::Text | State::Equals | State::EqualsBlanks => (),
            State::Cr => self.clean = false,
            State::Hex(high) => {
                self.queue.push(&[b'=', high]);
                self.clean = false;
            }
            State::EqualsCr => {
                self.queue.push(b"=");
                self.clean = false;
            }
        }

        self.state = State::Text;
        self.blanks_len = 0;

        if dst.len() < self.queue.len() {
            return None;
        }

        Some(QuotedPrintableDecodeOutput {
            read: 0,
            written: self.queue.drain(dst),
            clean: self.clean,
        })
    }

    /// Decodes one byte into the queue, a byte that ends a malformed escape
    /// being decoded again as data.
    fn step(&mut self, byte: u8) {
        loop {
            match (self.state, byte) {
                (State::Text, b' ' | b'\t') => {
                    if self.blanks_len == BLANKS_MAX {
                        self.flush_blanks();
                    }

                    self.push_blank(byte);
                }
                (State::Text, b'=') => {
                    self.flush_blanks();
                    self.state = State::Equals;
                }
                (State::Text, b'\r') => self.state = State::Cr,
                (State::Text, b'\n') => {
                    self.blanks_len = 0;
                    self.queue.push(b"\n");
                }
                (State::Text, _) => {
                    self.flush_blanks();
                    self.clean &= matches!(byte, b'!'..=b'~');
                    self.queue.push(&[byte]);
                }
                (State::Cr, b'\n') => {
                    self.blanks_len = 0;
                    self.queue.push(b"\r\n");
                    self.state = State::Text;
                }
                (State::Cr, _) => {
                    self.clean = false;
                    self.state = State::Text;
                    continue;
                }
                (State::Equals, b'0'..=b'9' | b'A'..=b'F' | b'a'..=b'f') => {
                    self.state = State::Hex(byte);
                }
                (State::Equals, b' ' | b'\t') => {
                    self.push_blank(byte);
                    self.state = State::EqualsBlanks;
                }
                (State::Equals, b'\r') => self.state = State::EqualsCr,
                (State::Equals | State::EqualsBlanks, b'\n') => self.soft_break(),
                (State::Equals, _) => {
                    self.malformed(b"=", State::Text);
                    continue;
                }
                (State::Hex(high), _) => match (hex(high), hex(byte)) {
                    (Some(high_nibble), Some(low_nibble)) => {
                        self.clean &= !high.is_ascii_lowercase() && !byte.is_ascii_lowercase();
                        self.queue.push(&[high_nibble << 4 | low_nibble]);
                        self.state = State::Text;
                    }
                    _ => {
                        self.malformed(&[b'=', high], State::Text);
                        continue;
                    }
                },
                (State::EqualsBlanks, b' ' | b'\t') => {
                    if self.blanks_len == BLANKS_MAX {
                        self.malformed(b"=", State::Text);
                        continue;
                    }

                    self.push_blank(byte);
                }
                (State::EqualsBlanks, b'\r') => self.state = State::EqualsCr,
                (State::EqualsBlanks, _) => {
                    self.malformed(b"=", State::Text);
                    continue;
                }
                (State::EqualsCr, b'\n') => self.soft_break(),
                (State::EqualsCr, _) => {
                    self.malformed(b"=", State::Cr);
                    continue;
                }
            }

            return;
        }
    }

    fn push_blank(&mut self, byte: u8) {
        self.blanks[self.blanks_len] = byte;
        self.blanks_len += 1;
    }

    /// Keeps the carried blanks as data, a byte other than a line break
    /// following them.
    fn flush_blanks(&mut self) {
        self.queue.push(&self.blanks[..self.blanks_len]);
        self.blanks_len = 0;
    }

    fn soft_break(&mut self) {
        self.blanks_len = 0;
        self.state = State::Text;
    }

    /// Keeps the bytes of a malformed escape literally and goes on from
    /// `state`, the blanks after its `=` carried as data.
    fn malformed(&mut self, bytes: &[u8], state: State) {
        self.queue.push(bytes);
        self.clean = false;
        self.state = state;
    }
}

/// The value of a hex digit, either case.
const fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
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
/// [`decoded_len_max`] asks.
pub fn decode(src: &[u8], dst: &mut [u8]) -> Option<QuotedPrintableDecodeOutput> {
    let dst = dst.get_mut(..decoded_len_max(src.len()))?;
    let mut decoder = QuotedPrintableDecode::new();
    let output = decoder.resume(src, dst);
    let end = decoder.finish(&mut dst[output.written..])?;

    Some(QuotedPrintableDecodeOutput {
        read: output.read,
        written: output.written + end.written,
        clean: end.clean,
    })
}

/// Decodes the whole of `src` at the end of `out`, returning whether it was
/// well-formed.
#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
pub fn decode_vec(src: &[u8], out: &mut Vec<u8>) -> bool {
    let start = out.len();
    out.resize(start + decoded_len_max(src.len()), 0);

    let decoded = decode(src, &mut out[start..]).unwrap_or(QuotedPrintableDecodeOutput {
        read: 0,
        written: 0,
        clean: false,
    });

    out.truncate(start + decoded.written);
    decoded.clean
}

/// The most bytes [`decode`] writes for an input of `len` bytes, whatever
/// they hold: every output byte comes from at least one input byte.
pub const fn decoded_len_max(len: usize) -> usize {
    len
}

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
        let mut expected = vec![b' '; BLANKS_MAX];
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
    fn every_split_gives_the_same_output() {
        let input = b"a =4\r\n=\t\r\nb=c3=A9 \r\nc= \rd=\r\n\te \t";
        let whole = decoded(input);

        for at in 0..=input.len() {
            let mut decoder = QuotedPrintableDecode::new();
            let mut out = vec![0; input.len()];
            let first = decoder.resume(&input[..at], &mut out);
            let second = decoder.resume(&input[at..], &mut out[first.written..]);
            let written = first.written + second.written;
            let end = decoder.finish(&mut out[written..]).unwrap();
            out.truncate(written + end.written);
            assert_eq!((out, end.clean), whole);
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
