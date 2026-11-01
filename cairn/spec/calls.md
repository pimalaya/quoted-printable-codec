---
cairn: spec
capability: calls
status: current
---

# Calls

How encoding and decoding are called: I/O-free, over the caller's buffers, chunk by chunk through `QuotedPrintableEncode` and `QuotedPrintableDecode` or at once through the one-shot functions built on them, with no dependency and no allocation outside the `alloc` helpers.

### Requirement: Lengths and short buffers

`encoded_len_max` SHALL bound what `encode` writes, or return none when that overflows `usize`; `decoded_len_max` SHALL bound what `decode` writes. `encode` and `decode` SHALL return none and write nothing when `dst` is shorter than their length helper asks.

### Requirement: Resumable without I/O

`QuotedPrintableEncode` and `QuotedPrintableDecode` SHALL give, for any split of the input into chunks and any output space offered, the same bytes and the same clean flag as the one-shot call, `finish` ending the input. `resume` SHALL always progress with one byte of room, and `finish` SHALL write nothing and return none when its output does not fit.

### Requirement: No dependency, no std

The library SHALL build `#![no_std]` with no dependency and no unsafe code, `alloc` serving only `encode_vec` and `decode_vec`.
