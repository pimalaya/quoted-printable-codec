---
cairn: delta
change: the-installed-token
---

## ADDED Requirements

## MODIFIED Requirements

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

## REMOVED Requirements
