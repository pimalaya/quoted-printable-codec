---
cairn: delta
change: the-narrow-vectors
---

## MODIFIED Requirements

### Requirement: The widest backend the CPU has

`QuotedPrintableCpu::new` SHALL name the AVX-512 backend when its `SimdCpu` holds AVX-512 VBMI and VBMI2, BMI1, BMI2, LZCNT and POPCNT, else the AVX2 backend when it holds AVX2, BMI1, LZCNT and POPCNT, on x86_64; on aarch64 built with NEON, the NEON backend when it holds NEON; and the portable one otherwise.

#### Scenario: A CPU with AVX-512 VBMI2
- GIVEN a `SimdCpu` detected on an x86_64 CPU with AVX-512 VBMI2, such as Zen 4 and later or Ice Lake and later
- WHEN the application builds a `QuotedPrintableCpu` from it
- THEN the token names the AVX-512 backend, without `Avx512Vbmi2` the AVX2 one, and from `SimdCpu::NONE` the portable one

#### Scenario: An aarch64 CPU
- GIVEN a `SimdCpu` detected on an aarch64 CPU, or `SimdCpu::BASELINE` of an aarch64 Linux build
- WHEN the application builds a `QuotedPrintableCpu` from it
- THEN the token names the NEON backend

### Requirement: Every backend matches the portable one

For every input, mode and line ending, split into any chunks with any output room, every backend SHALL write the same bytes and return the same clean flag as the portable backend.

#### Scenario: Random bodies under every backend
- GIVEN 2,000 random bodies of up to 300 bytes, drawn mostly from the bytes both directions treat apart
- WHEN they are encoded in every mode and line ending, and decoded raw and encoded, in one call and resumed in random chunks with random room, under every backend the CPU supports
- THEN every backend writes the bytes and the flag of the portable one, and the frozen corpus keeps its hash under each

#### Scenario: Fuzzed inputs under every backend
- GIVEN any bytes, mode, line ending, chunk size and room a fuzzer draws
- WHEN the encode and decode targets run them under every backend the machine has, in one call and resumed
- THEN every backend gives the bytes and the flag of the portable one, and the portable encoding decodes back clean, line breaks as the mode writes them, in lines of at most 76 characters

### Requirement: One dependency, no std

The library SHALL build `#![no_std]` under every feature combination with simd-cpu, without its `std` feature, as its only dependency, `alloc` serving only `QuotedPrintableCodec::encode_vec` and `QuotedPrintableCodec::decode_vec`. Unsafe code SHALL serve only the SIMD backends: their loads and stores and the calls into them a token allows.
