# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-11-03

### Added

- Added `codec::QuotedPrintableCodec`, the `Copy` value every call runs from.

  It holds the backend (`cpu`), the line ending (`eol`), the mode (`mode`) and the line-start guard (`guard`). `BASELINE` needs no CPU detection.

- Added `codec::QuotedPrintableMode`: MIME body text, binary MIME body content, and the RFC 2047 Q encoding of header words.
- Added `codec::QuotedPrintableEol`, ending encoded lines of at most 76 characters with CRLF or LF.
- Added `QuotedPrintableCodec::guard`, escaping a `.`, `F` or `-` that would start an encoded line (RFC 2049 section 3).
- Added `QuotedPrintableCodec::encode` and `QuotedPrintableCodec::decode`, the one-shot calls.
- Added `QuotedPrintableCodec::encode_vec` and `QuotedPrintableCodec::decode_vec`, behind the `alloc` feature.
- Added `QuotedPrintableCodec::encoded_len_max` and `QuotedPrintableCodec::decoded_len_max`, the output lengths to allocate.
- Added `QuotedPrintableCodec::encoded_len`, the exact encoded length.
- Added `encode::QuotedPrintableEncode`, the I/O-free encoder over the caller's buffers, the input cut anywhere.
- Added `decode::QuotedPrintableDecode`, the I/O-free decoder over the caller's buffers, the input cut anywhere.

  Decoding never fails and follows the robust rules of RFC 2045 section 6.7: malformed escapes and bare CRs are kept and reported through `clean`.

- Added `cpu::QuotedPrintableCpu`, the widest backend a `simd_cpu::SimdCpu` token has the features of.

  `QuotedPrintableCpu::default()` and `QuotedPrintableCodec::default()` run the token the application installed with `SimdCpu::install_detected`, the build target's baseline when none is.

- Added the portable, AVX2, AVX-512 VBMI2 and NEON backends, all writing the same bytes.

[Unreleased]: https://github.com/pimalaya/quoted-printable-codec/compare/v0.1.0..HEAD
[0.1.0]: https://github.com/pimalaya/quoted-printable-codec/compare/root..v0.1.0
