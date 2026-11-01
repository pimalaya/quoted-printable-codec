---
cairn: log
change: a-codec-value
landed: 2026-11-01
---

# A codec value

The user asked for base64-codec's a-codec-value here too: the one-shot calls as methods of a codec value, and the options renamed into it, so both crates read alike.

What landed:

- `codec::QuotedPrintableCodec`, a `Copy` value with the public fields `eol` and `binary`, defaulting to text with CRLF; `QuotedPrintableEol` and its `soft_break` moved next to it. No `BASELINE` constant: there is no CPU token, and a `static` writes the struct out.
- The one-shot calls are its methods, taking `self`: `encode`, `encode_vec` and `encoded_len_max` in encode.rs, `decode`, `decode_vec` and `decoded_len_max` in decode.rs. The free functions and `QuotedPrintableEncodeOptions` are gone. `encode` and `decode` are `#[inline]`, as base64-codec's, so the caller goes straight to `blocks` or `tokens`.
- `QuotedPrintableEncode::new` takes a codec; `QuotedPrintableDecode::new` takes one too and ignores it, as decoding takes any input, so that one value serves every call and a decoding option would not change the signature.
- The tests, the bench, the crate and module docs on the new calls.

Output unchanged: `a_frozen_corpus_keeps_its_output` holds its hash. criterion, pinned to core 13, the tree before in brackets, encoding at 4 KiB over two runs each: ascii 947 and 951 ns (984, 988), accented 2.58 and 2.60 µs (2.54, 2.53), binary 4.68 and 4.63 µs (5.01, 4.94); decoding in one run: 64 B ascii 20.6 ns (20.0), accented 43.1 ns (44.5), binary 75.1 ns (87.2); 4 KiB ascii 780 ns (819), accented 2.31 µs (2.38), binary 4.32 µs (4.35). Within the layout noise the-small-calls measured; a first run read encoding accented at 4 KiB 54% slower and did not repeat. No `perf` counts this time. The code: 531 lines against 526 (non-blank, non-comment lines before each `#[cfg(test)]`, after `cargo fmt`), the codec module.

Checked: the test suite in release and debug with all features, the library without default features, clippy for all targets with `-D warnings`, rustdoc with `-D warnings`, the bench built.

Capabilities moved: calls (one codec for every call, the length and `alloc` helpers on it).
