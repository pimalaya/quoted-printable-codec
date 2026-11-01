---
cairn: log
change: a-shared-cpu-token
landed: 2026-11-01
---

# A shared CPU token

The user settled on 2026-11-01 that every SIMD codec takes `simd_cpu::SimdCpu`, a token of CPU features from the new simd-cpu crate (../simd-cpu, its log 2026-11-01-a-simd-cpu-token.md), detected once by the application; base64-codec moved to it the same day. This crate followed once its AVX-512 pass was committed (72d8dc6), the user asking for it then.

What landed:

- simd-cpu 0.0.1 as the one dependency, pinned by git revision to 4cde2e9 on GitHub, and with its `std` feature as a dev-dependency; the `std` feature of this crate is gone, and so is the bench's `required-features`.
- `QuotedPrintableCpu::new(SimdCpu)`, a `const fn`: the AVX-512 backend when the token holds AVX-512 VBMI2 and VBMI, BMI1, BMI2, LZCNT and POPCNT, the portable one otherwise. `BASELINE`, `assume`, `detect` and `detect_avx512` are gone; `backend`, `wide` and `QuotedPrintableBackend` stay. The pass before had already added the BMI1 check the proposal asked for.
- `Default` builds the token in a `const` block. Calling `new` there at run time left the code the same but pushed `QuotedPrintableDecode::new`, which takes the default, past rustc's cross-crate inlining threshold: the library emitted it as a symbol of its own, so that other crates would have called it out of line. With the `const` block it is gone again; base64-codec's `Default` took the same form.
- The test-only `available()` lists `new(SimdCpu::NONE)` then `new(SimdCpu::detect())` when they differ, and the bench builds its token from `SimdCpu::detect()`: the token module has no unsafe code left but the `wide` call into the AVX-512 blocks.
- `wide` takes `#[expect(unused_variables)]` off x86_64: its `src` went unused on aarch64 since the pass before, a warning there that clippy with `-D warnings` turned into an error. The CI runs on x86_64 only, which hid it.
- The crate docs, README, benches/README, Cargo, flake and package descriptions lose "dependency-free"; the-wider-reach's handoff marks lever 0 done and points its harness at `QuotedPrintableCpu::new` (the scratch harness of 2026-11-01 still calls `assume` and `detect`).

The release assembly of the library against the committed tree, symbol hashes and local labels normalized, function by function: all 55 functions identical, one added, the cold `QuotedPrintableCpu::new` (`not edi; test dil, -4; sete al; ret`). The cycle counts of the pass before stand.

Checked: the test suite in release and debug with all features, without default features and with `-C target-cpu=native`, on this Zen 5 machine both backends (`available` gives portable and AVX-512); clippy with `-D warnings` for all targets and for the library without default features, on x86_64 and on aarch64-unknown-linux-musl; the aarch64 suite in release under qemu-aarch64 on neoverse-n1; missing docs and rustdoc with `-D warnings`; the bench built, not run.

Capabilities moved: backends (the token built from a `SimdCpu`), calls (one dependency, no std).
