---
cairn: delta
change: the-wide-blocks
---

## ADDED Requirements

### Requirement: CPU detection is the caller's

No encoding or decoding function SHALL detect CPU features. A backend SHALL run only from a `QuotedPrintableCpu` token: `BASELINE`, the best backend the build target guarantees through `cfg(target_feature)`; `assume`, unsafe; or `detect`, behind the `std` feature.

#### Scenario: A default x86_64 build without std
- GIVEN the crate built for x86_64 with no feature and no `-C target-feature`
- WHEN `QuotedPrintableCodec::default()` encodes or decodes an input
- THEN it runs the portable backend, and no detection code is compiled

### Requirement: The widest backend the CPU has

`QuotedPrintableCpu::detect` SHALL name the AVX-512 backend when the running CPU has AVX-512 VBMI and VBMI2, BMI1, BMI2, LZCNT and POPCNT, and the portable one otherwise; `QuotedPrintableCpu::BASELINE` SHALL name the AVX-512 backend only when the build target's features guarantee all of them.

#### Scenario: A CPU with AVX-512 VBMI2
- GIVEN an x86_64 CPU with AVX-512 VBMI2, such as Zen 4 and later or Ice Lake and later
- WHEN the application calls `QuotedPrintableCpu::detect`
- THEN the token names the AVX-512 backend

### Requirement: Every backend matches the portable one

For every input, mode and line ending, split into any chunks with any output room, every backend SHALL write the same bytes and return the same clean flag as the portable backend.

#### Scenario: Random bodies under every backend
- GIVEN 2,000 random bodies of up to 300 bytes, drawn mostly from the bytes both directions treat apart
- WHEN they are encoded in every mode and line ending, and decoded raw and encoded, in one call and resumed in random chunks with random room, under every backend the CPU supports
- THEN every backend writes the bytes and the flag of the portable one, and the frozen corpus keeps its hash under each

## MODIFIED Requirements

### Requirement: Lengths and short buffers

`QuotedPrintableCodec::encoded_len_max` SHALL give the room `QuotedPrintableCodec::encode` needs, a bound on what it writes plus the 200 bytes its blocks may touch past the output, or return none when that overflows `usize`; `QuotedPrintableCodec::decoded_len_max` SHALL bound what `QuotedPrintableCodec::decode` writes. `encode` and `decode` SHALL return none and write nothing when `dst` is shorter than their length helper asks. Every call MAY overwrite the bytes of `dst` past the ones it reports written, within the length it was given.

### Requirement: No dependency, no std

The library SHALL build `#![no_std]` with no dependency under every feature combination, `std` serving only `QuotedPrintableCpu::detect` and `alloc` only `QuotedPrintableCodec::encode_vec` and `QuotedPrintableCodec::decode_vec`. Unsafe code SHALL serve only the AVX-512 backend: its loads and stores, the calls into it a token allows, and `QuotedPrintableCpu::assume`.

### Requirement: One codec for every call

Every call SHALL run from a `QuotedPrintableCodec`, a `Copy` value holding the backend token, the mode (text or binary) and the line ending of encoded output: its one-shot methods encode or decode a whole input, and `QuotedPrintableEncode::new` and `QuotedPrintableDecode::new` take it to encode or decode chunk by chunk. The codec SHALL hold no state between calls, so that no one-shot call can run in the middle of a resumed input. Decoding SHALL take any input whatever the codec's mode and line ending.

#### Scenario: One codec for the whole application
- GIVEN a codec built once in binary mode with LF line endings
- WHEN it encodes an input in one call, and a `QuotedPrintableEncode` built from it encodes the same input chunk by chunk, then finishes
- THEN both write the same bytes
