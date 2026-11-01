---
cairn: spec
capability: backends
status: current
---

# Backends

Every call runs the backend the `QuotedPrintableCpu` token of its codec names: portable on every target, AVX-512 on x86_64. The portable backend is safe code walking the special bytes of blocks of 32 bytes, its masks vectorized by the compiler; the AVX-512 one runs blocks of 64 bytes with no branch on the data, decoding with the VBMI2 byte compress and encoding by spreading each byte over the three lanes of its escape. Both leave the same tokens to the same scalar steps, and the encoder runs the portable walk on a short input with few bytes to escape whatever the token, where that walk is faster.

### Requirement: CPU detection is the caller's

No encoding or decoding function SHALL detect CPU features. A backend SHALL run only from a `QuotedPrintableCpu` that `QuotedPrintableCpu::new` built from a `simd_cpu::SimdCpu`, the application's token of the features the running CPU has.

#### Scenario: A default x86_64 build without std
- GIVEN the crate built for x86_64 with no feature and no `-C target-feature`
- WHEN `QuotedPrintableCodec::default()` encodes or decodes an input
- THEN it runs the portable backend, and no detection code is compiled

### Requirement: The widest backend the CPU has

`QuotedPrintableCpu::new` SHALL name the AVX-512 backend when its `SimdCpu` holds AVX-512 VBMI and VBMI2, BMI1, BMI2, LZCNT and POPCNT, and the portable one otherwise.

#### Scenario: A CPU with AVX-512 VBMI2
- GIVEN a `SimdCpu` detected on an x86_64 CPU with AVX-512 VBMI2, such as Zen 4 and later or Ice Lake and later
- WHEN the application builds a `QuotedPrintableCpu` from it
- THEN the token names the AVX-512 backend, and from `SimdCpu::NONE` the portable one

### Requirement: Every backend matches the portable one

For every input, mode and line ending, split into any chunks with any output room, every backend SHALL write the same bytes and return the same clean flag as the portable backend.

#### Scenario: Random bodies under every backend
- GIVEN 2,000 random bodies of up to 300 bytes, drawn mostly from the bytes both directions treat apart
- WHEN they are encoded in every mode and line ending, and decoded raw and encoded, in one call and resumed in random chunks with random room, under every backend the CPU supports
- THEN every backend writes the bytes and the flag of the portable one, and the frozen corpus keeps its hash under each
