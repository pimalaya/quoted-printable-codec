---
cairn: change
id: the-narrow-vectors
status: landed
created: 2026-11-02
---

# The narrow vectors

## Why

Only CPUs with AVX-512 VBMI2 run a SIMD backend today: the x86_64 CPUs without it (Intel client CPUs since Alder Lake, AMD before Zen 4) and every aarch64 machine run the portable one. The handoff the-column-chain lists an AVX2 backend, a NEON backend, Miri and fuzz targets among its levers (2, 3 and 7). On 2026-11-02 the user asked for the missing backends with accuracy first, their speed left to the passes after, and for Miri and fuzz targets.

The same day the user asked what the Q and DKIM modes of that handoff are. The answer, with the recommendation to add neither, is in the log of this change. The decision is still open, and neither mode is part of this change.

## What

- `QuotedPrintableBackend::Avx2` on x86_64, for a token holding AVX2, BMI1, LZCNT and POPCNT, and `QuotedPrintableBackend::Neon` on aarch64 builds with NEON. `QuotedPrintableCpu::new` picks AVX-512, then AVX2, then portable on x86_64, and NEON, then portable on aarch64.
- src/avx2.rs and src/neon.rs take the steps of the AVX-512 kernels on narrower vectors, with no `pext` or `pdep`, which Zen 2 runs in microcode:
  - Decoding runs blocks of 32 bytes (AVX2) or 16 (NEON) with the same masks: escapes decoded and blended in, digits and soft breaks compressed away 8 lanes at a time through a table of 256 shuffles, the block cut at its first slow byte.
  - Encoding runs groups of 16 bytes: the bytes each byte takes, spread over three lanes by table shuffles, with unused lanes marked by their high bit and compressed away. The one soft break a group may need comes from the output offsets of its bytes, counted from the masks of escapes, line feeds and dropped CRs.
- The encoder dispatch moves into `blocks`, and `QuotedPrintableCpu::wide` goes; only the AVX-512 backend keeps the walk for short inputs with few escapes.
- The tests list every backend the CPU has, portable first. Under Miri they run fewer bodies and skip the frozen corpus.
- fuzz/ holds two cargo-fuzz targets, encode and decode, comparing every backend the machine has with the portable one, one-shot and resumed. The portable encoding must also decode back clean, in lines of at most 76 characters.
- A nightly shell, fuzz/shell.nix, for Miri and cargo-fuzz; CI jobs for Miri, the fuzz targets and the aarch64 suite under qemu.

Out of scope: the speed of the new backends, which later passes measure and tune (the-column-chain stays their handoff), and the Q and DKIM modes.
