---
cairn: change
id: a-shared-cpu-token
status: landed
created: 2026-11-01
---

# A shared CPU token

## Why

base64-codec and this crate each carried their own CPU detection behind a `std` feature, and base32, UTF-7 and later codecs would carry more. The new crate simd-cpu (~/code/pimalaya/simd-cpu, its cairn log 2026-11-01-a-simd-cpu-token.md) holds the one token an application detects once and hands to every codec; base64-codec moved to it on 2026-11-01 (its a-shared-cpu-token).

Applied once the AVX-512 pass (the-wider-reach, perfs pass 4) was committed.

## What

- `simd-cpu` 0.0.1, pinned by git revision, as the one dependency, and with `features = ["std"]` as a dev-dependency; the `std` feature goes.
- `QuotedPrintableCpu::new(cpu: SimdCpu)`, a `const fn`: the AVX-512 backend when `cpu` holds `Avx512Vbmi2`, `Avx512Vbmi`, `Bmi1`, `Bmi2`, `Lzcnt` and `Popcnt`, the portable one otherwise. `BASELINE`, `detect`, `assume` and `detect_avx512` go; `Default`, and so the codec's default, builds from `SimdCpu::BASELINE` in a `const` block; `backend`, `wide` and `QuotedPrintableBackend` stay.
- The test-only `available()` gives `new(SimdCpu::NONE)` and `new(SimdCpu::detect())`, and the bench builds its token from `SimdCpu::detect()`: no unsafe constructor left.
- The crate docs, the README and the nix and Cargo descriptions lose "dependency-free".
- Check that the release assembly of the library is unchanged, symbol hashes normalized, as base64-codec's was: the token keeps its layout and its enum.
