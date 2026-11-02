//! # Codec
//!
//! [`QuotedPrintableCodec`], the configuration every call runs. Its one-shot
//! methods live in [`encode`](crate::encode) and [`decode`](crate::decode),
//! whose coroutines take it too.

use simd_cpu::SimdCpu;

use crate::cpu::QuotedPrintableCpu;

/// The backend, the mode and the line ending of encoded output.
///
/// A stateless value, built once and used for every call. Defaults to text
/// with CRLF on the installed token's backend. Another token goes in
/// [`cpu`](Self::cpu), built with `QuotedPrintableCpu::new`.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct QuotedPrintableCodec {
    /// The backend every call runs.
    pub cpu: QuotedPrintableCpu,
    /// The line ending of hard and soft line breaks. Decoding takes either.
    pub eol: QuotedPrintableEol,
    /// MIME body text or binary, or RFC 2047 encoded-word text.
    pub mode: QuotedPrintableMode,
}

impl QuotedPrintableCodec {
    /// Text with CRLF on the build target's backend, ignoring the installed
    /// token.
    pub const BASELINE: Self = Self {
        cpu: QuotedPrintableCpu::new(SimdCpu::BASELINE),
        eol: QuotedPrintableEol::Crlf,
        mode: QuotedPrintableMode::Text,
    };
}

/// What a codec encodes and decodes.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum QuotedPrintableMode {
    /// MIME body text: LF and CRLF are hard breaks in the codec's line ending.
    #[default]
    Text,
    /// Binary MIME body content, such as attachments: CR and LF are escaped.
    Binary,
    /// The RFC 2047 Q encoding of encoded-word text, in header fields.
    ///
    /// Letters, digits and `!*+-/` stay literal, a space becomes `_`, other
    /// bytes are escaped. No line breaks: the caller splits the words.
    /// Decoding reports only a `=` starting no escape, as words in the wild
    /// carry raw bytes and lowercase digits.
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
    /// The soft break, `=` and the line ending padded to 4 bytes, and its
    /// length.
    pub(crate) const fn soft_break(self) -> ([u8; 4], usize) {
        match self {
            Self::Crlf => (*b"=\r\n\0", 3),
            Self::Lf => (*b"=\n\0\0", 2),
        }
    }
}
