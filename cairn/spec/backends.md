---
cairn: spec
capability: backends
status: current
---

# Backends

Every call runs the backend the `QuotedPrintableCpu` token of its codec names: portable on every target, AVX-512 and AVX2 on x86_64, NEON on aarch64. The portable backend is safe code walking the special bytes of blocks of 32 bytes, its masks vectorized by the compiler. The AVX-512 one runs blocks of 64 bytes with no branch on the data, decoding with the VBMI2 byte compress and encoding by spreading each byte over the three lanes of its escape. The AVX2 and NEON ones take its steps on blocks of 32 and 16 bytes, compressing bytes 8 lanes at a time through a table of shuffles, encoding groups of 16 bytes, and placing the soft break from offsets counted from the masks of a group's bytes. Every backend leaves the same tokens to the same scalar steps. On the AVX-512 token, the encoder runs the portable walk on a short input with few bytes to escape, where that walk is faster. Q mode runs its scalar steps on every token: an encoded word holds 75 characters at most, too few for SIMD to matter.

### Requirement: CPU detection is the caller's

No encoding or decoding function SHALL detect CPU features. A backend SHALL run only from a `QuotedPrintableCpu` that `QuotedPrintableCpu::new` built from a `simd_cpu::SimdCpu`: one the caller gives, or, for `QuotedPrintableCpu::default()` and `QuotedPrintableCodec::default()`, `SimdCpu::current()`, the token the application installed or `SimdCpu::BASELINE`, read once when the value is built. `QuotedPrintableCodec::BASELINE` SHALL run the backend of `SimdCpu::BASELINE` whatever is installed.

#### Scenario: A default x86_64 build without std
- GIVEN the crate built for x86_64 with no feature and no `-C target-feature`, and no token installed
- WHEN `QuotedPrintableCodec::default()` encodes or decodes an input
- THEN it runs the portable backend, and no detection code is compiled

#### Scenario: A default codec and the installed token
- GIVEN a process on a CPU with AVX2, BMI1, LZCNT and POPCNT, and a `QuotedPrintableCodec::default()` built before any install
- WHEN the application calls `SimdCpu::install_detected()`, then builds another `QuotedPrintableCodec::default()`
- THEN the first codec runs the portable backend, the second the AVX2 backend or a wider one

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
