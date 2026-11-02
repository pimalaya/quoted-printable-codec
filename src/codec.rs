//! # Codec
//!
//! [`QuotedPrintableCodec`], the configuration every call runs: the backend,
//! the mode (MIME body text or binary, or the Q encoding of header words)
//! and the line ending of encoded output. Its one-shot methods, in
//! [`encode`](crate::encode) and [`decode`](crate::decode), encode and decode
//! a whole input; the coroutines of those modules take it to work chunk by
//! chunk.

use simd_cpu::SimdCpu;

use crate::cpu::QuotedPrintableCpu;

/// The backend, the mode and the line ending of encoded output.
///
/// A plain value holding no state: an application builds one and uses it
/// for every call, its one-shot methods as the encoders and decoders built
/// from it. Defaults to text with CRLF on the backend of the token the
/// application installed, read when the value is built; an explicit token
/// goes in [`cpu`](Self::cpu): `QuotedPrintableCodec { cpu:
/// QuotedPrintableCpu::new(token), ..QuotedPrintableCodec::default() }`.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct QuotedPrintableCodec {
    /// The backend every call runs.
    pub cpu: QuotedPrintableCpu,
    /// The line ending of hard and soft line breaks. Decoding takes either.
    pub eol: QuotedPrintableEol,
    /// What the bytes are: the text or binary content of a MIME body, or the
    /// text of RFC 2047 encoded words.
    pub mode: QuotedPrintableMode,
}

impl QuotedPrintableCodec {
    /// The build target's backend, whatever the application installed, text
    /// with CRLF.
    pub const BASELINE: Self = Self {
        cpu: QuotedPrintableCpu::new(SimdCpu::BASELINE),
        eol: QuotedPrintableEol::Crlf,
        mode: QuotedPrintableMode::Text,
    };
}

/// What a codec encodes and decodes.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum QuotedPrintableMode {
    /// The text of a MIME body: its LF and CRLF line breaks are hard line
    /// breaks, written in the codec's line ending.
    #[default]
    Text,
    /// The binary content of a MIME body, such as an attachment: CR and LF
    /// are escaped as data.
    Binary,
    /// The RFC 2047 4.2 Q encoding of the text of encoded words, in header
    /// fields: letters, digits and `!*+-/` literal, a space as `_`, any other
    /// byte escaped, and no line break, as the caller splits the text into
    /// encoded words; decoding reads `_` as a space and reports only a `=`
    /// starting no escape, as words in the wild carry raw bytes and lowercase
    /// digits.
    ///
    /// Refs: <https://datatracker.ietf.org/doc/html/rfc2047#section-4.2>
    Q,
}

/// The line ending of the output.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum QuotedPrintableEol {
    /// CR LF, as the wire format of mail has it.
    #[default]
    Crlf,
    /// LF alone, as mail stored on disk often has it.
    Lf,
}

impl QuotedPrintableEol {
    /// The soft line break, `=` and the line ending, padded to 4 bytes, and
    /// its length.
    pub(crate) const fn soft_break(self) -> ([u8; 4], usize) {
        match self {
            Self::Crlf => (*b"=\r\n\0", 3),
            Self::Lf => (*b"=\n\0\0", 2),
        }
    }
}
