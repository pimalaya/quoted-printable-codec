---
cairn: log
change: a-quoted-printable-codec
landed: 2026-11-01
---

# A quoted-printable codec

Started the crate on base64-codec's shape: the resumable `QuotedPrintableEncode` and `QuotedPrintableDecode` with the one-shot `encode` and `decode` over them, the length helpers and the `alloc` helpers, the Nix files, licenses and CI. Scalar only, byte by byte through a small output queue, with encodify 1.0.2's body rules (src/qp/encode.rs and src/qp/decode.rs) as the reference; no unsafe code.

Left out for now: a `QuotedPrintableCpu` token and SIMD backends (simdutf8's driver structure is the plan when speed matters), RFC 2047 Q and DKIM modes, an exact `encoded_len`, fuzz targets, benches and ATTRIBUTION.md.

Capabilities moved: encoding, decoding, calls (all ADDED).
