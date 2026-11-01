---
cairn: spec
capability: calls
status: current
---

# Calls

How encoding and decoding are called: from one `QuotedPrintableCodec` value, I/O-free, over the caller's buffers, at once through its one-shot methods, which run the same blocks, or chunk by chunk through the `QuotedPrintableEncode` and `QuotedPrintableDecode` built from it, with no dependency and no allocation outside the `alloc` helpers.

### Requirement: Lengths and short buffers

`QuotedPrintableCodec::encoded_len_max` SHALL bound what `QuotedPrintableCodec::encode` writes, or return none when that overflows `usize`; `QuotedPrintableCodec::decoded_len_max` SHALL bound what `QuotedPrintableCodec::decode` writes. `encode` and `decode` SHALL return none and write nothing when `dst` is shorter than their length helper asks. Every call MAY overwrite the bytes of `dst` past the ones it reports written, within the length it was given.

### Requirement: Resumable without I/O

`QuotedPrintableEncode` and `QuotedPrintableDecode` SHALL give, for any split of the input into chunks and any output space offered, the same bytes and the same clean flag as the one-shot call, `finish` ending the input. `resume` SHALL always progress with one byte of room, and `finish` SHALL write nothing and return none when its output does not fit.

### Requirement: No dependency, no std

The library SHALL build `#![no_std]` with no dependency and no unsafe code, `alloc` serving only `QuotedPrintableCodec::encode_vec` and `QuotedPrintableCodec::decode_vec`, and no CPU feature beyond the target's baseline.

### Requirement: One codec for every call

Every call SHALL run from a `QuotedPrintableCodec`, a `Copy` value holding the mode (text or binary) and the line ending of encoded output: its one-shot methods encode or decode a whole input, and `QuotedPrintableEncode::new` and `QuotedPrintableDecode::new` take it to encode or decode chunk by chunk. The codec SHALL hold no state between calls, so that no one-shot call can run in the middle of a resumed input. Decoding SHALL take any input whatever the codec's mode and line ending.

#### Scenario: One codec for the whole application
- GIVEN a codec built once in binary mode with LF line endings
- WHEN it encodes an input in one call, and a `QuotedPrintableEncode` built from it encodes the same input chunk by chunk, then finishes
- THEN both write the same bytes
