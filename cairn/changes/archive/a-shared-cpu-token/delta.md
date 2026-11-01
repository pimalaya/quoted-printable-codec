---
cairn: delta
change: a-shared-cpu-token
---

## ADDED Requirements

### Requirement: One dependency, no std

The library SHALL build `#![no_std]` under every feature combination with simd-cpu, without its `std` feature, as its only dependency, `alloc` serving only `QuotedPrintableCodec::encode_vec` and `QuotedPrintableCodec::decode_vec`. Unsafe code SHALL serve only the AVX-512 backend: its loads and stores and the calls into it a token allows.

## MODIFIED Requirements

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

## REMOVED Requirements

### Requirement: No dependency, no std
