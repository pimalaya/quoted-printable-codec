//! # Codec
//!
//! [`QuotedPrintableCodec`], the configuration every call runs: the backend,
//! the mode and the line ending of encoded output. Its one-shot methods, in
//! [`encode`](crate::encode) and [`decode`](crate::decode), encode and decode
//! a whole input; the coroutines of those modules take it to work chunk by
//! chunk.

use crate::cpu::QuotedPrintableCpu;

/// The backend, the mode and the line ending of encoded output.
///
/// A plain value holding no state: an application builds one and uses it
/// for every call, its one-shot methods as the encoders and decoders built
/// from it. Defaults to the build's baseline backend, text with CRLF.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct QuotedPrintableCodec {
    /// The backend every call runs.
    pub cpu: QuotedPrintableCpu,
    /// The line ending of hard and soft line breaks. Decoding takes either.
    pub eol: QuotedPrintableEol,
    /// Whether CR and LF are escaped as data rather than read as line
    /// breaks, as binary content such as an attachment needs.
    pub binary: bool,
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
