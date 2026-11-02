---
cairn: spec
capability: calls
status: current
---

# Calls

How encoding and decoding are called: from one `QuotedPrintableCodec` value, I/O-free, over the caller's buffers, at once through its one-shot methods, which run the same blocks, or chunk by chunk through the `QuotedPrintableEncode` and `QuotedPrintableDecode` built from it, with simd-cpu as its one dependency and no allocation outside the `alloc` helpers; the backend its token names is in the backends capability.

### Requirement: Lengths and short buffers

`QuotedPrintableCodec::encoded_len_max` SHALL give the room `QuotedPrintableCodec::encode` needs, a bound on what it writes plus the 200 bytes its blocks may touch past the output, or return none when that overflows `usize`; `QuotedPrintableCodec::decoded_len_max` SHALL bound what `QuotedPrintableCodec::decode` writes. `encode` and `decode` SHALL return none and write nothing when `dst` is shorter than their length helper asks. Every call MAY overwrite the bytes of `dst` past the ones it reports written, within the length it was given.

### Requirement: The exact encoded length

`QuotedPrintableCodec::encoded_len` SHALL give the exact number of bytes `QuotedPrintableCodec::encode` writes for an input, in every mode.

### Requirement: Resumable without I/O

`QuotedPrintableEncode` and `QuotedPrintableDecode` SHALL give, for any split of the input into chunks and any output space offered, the same bytes and the same clean flag as the one-shot call, `finish` ending the input. `resume` SHALL always progress with one byte of room, and `finish` SHALL write nothing and return none when its output does not fit. A `finish` that succeeds SHALL leave the coroutine as new, ready for another input.

#### Scenario: A coroutine reused after finish
- GIVEN an encoder that encoded and finished one input, ending mid-line, and a decoder that decoded and finished an unclean input
- WHEN each takes another input and finishes again
- THEN each gives what a new one gives: the encoder breaks lines from column 0, and the decoder reports the second input clean when it is

### Requirement: One dependency, no std

The library SHALL build `#![no_std]` under every feature combination with simd-cpu, without its `std` feature, as its only dependency, `alloc` serving only `QuotedPrintableCodec::encode_vec` and `QuotedPrintableCodec::decode_vec`. Unsafe code SHALL serve only the SIMD backends: their loads and stores and the calls into them a token allows.

### Requirement: One codec for every call

Every call SHALL run from a `QuotedPrintableCodec`, a `Copy` value holding the backend token, the mode (text, binary or Q) and the line ending of encoded output: its one-shot methods encode or decode a whole input, and `QuotedPrintableEncode::new` and `QuotedPrintableDecode::new` take it to encode or decode chunk by chunk. The codec SHALL hold no state between calls, so that no one-shot call can run in the middle of a resumed input. Decoding SHALL take any input whatever the codec's line ending, as a MIME body in text and binary modes and as the text of encoded words in Q mode.

#### Scenario: One codec for the whole application
- GIVEN a codec built once in binary mode with LF line endings
- WHEN it encodes an input in one call, and a `QuotedPrintableEncode` built from it encodes the same input chunk by chunk, then finishes
- THEN both write the same bytes
