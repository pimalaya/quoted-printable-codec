//! # quoted-printable-codec
//!
//! Quoted-printable encoding and decoding of MIME bodies (RFC 2045 6.7),
//! lenient as mail needs it, `no_std`, with no dependency and no I/O.
//!
//! [`encode`] and [`decode`] hold the API: [`QuotedPrintableEncode`] and [`QuotedPrintableDecode`]
//! take the caller's input and output buffers chunk by chunk, as any I/O-free
//! coroutine does; the one-shot functions run them once over a whole input.
//!
//! The implementation is scalar, byte by byte: it follows the rules of
//! encodify 1.0.2's quoted-printable module, with none of its kernels yet.
//!
//! ```
//! use quoted_printable_codec::{
//!     decode::decode,
//!     encode::{QuotedPrintableEncodeOptions, encode, encoded_len_max},
//! };
//!
//! let input = "Grüße,\nJürgen\n".as_bytes();
//! let opts = QuotedPrintableEncodeOptions::default();
//!
//! let mut encoded = [0; 64];
//! assert!(encoded.len() >= encoded_len_max(input.len(), opts).unwrap());
//! let len = encode(input, &mut encoded, opts).unwrap();
//! assert_eq!(&encoded[..len], b"Gr=C3=BC=C3=9Fe,\r\nJ=C3=BCrgen\r\n");
//!
//! let mut decoded = [0; 64];
//! let output = decode(&encoded[..len], &mut decoded).unwrap();
//! assert_eq!(&decoded[..output.written], "Grüße,\r\nJürgen\r\n".as_bytes());
//! assert!(output.clean);
//! ```
//!
//! [`QuotedPrintableEncode`]: encode::QuotedPrintableEncode
//! [`QuotedPrintableDecode`]: decode::QuotedPrintableDecode

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", test))]
extern crate alloc;

pub mod decode;
pub mod encode;
mod queue;
