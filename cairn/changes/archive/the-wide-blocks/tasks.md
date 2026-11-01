---
cairn: tasks
change: the-wide-blocks
---

- [x] Rebuild the measuring harnesses (perf in both builds, before/after, fuzz) as the proposal describes
- [ ] 64-lane blocks: the 1 MiB and 4 KiB text gains without losing a 64-byte row or binary encoding: superseded; the user asked why SIMD was not used, and the AVX-512 backend runs 64-byte blocks with no branch on the data, 1.07x to 8x faster than these rows (log 2026-11-01-the-wide-blocks); the portable blocks stay at 32 lanes
- [x] Decoding binary at 64 bytes: a clear lead over encodify in both builds: 1.56 cycles a byte under AVX-512 against encodify's 6.02 (3.9x)
- [ ] Size: under 506 lines, no row lost: not reached, 820 lines, src/avx512.rs and src/cpu.rs 281 of them; moved to the-wider-reach
- [ ] aarch64: build and measure: moved to the-wider-reach (the portable backend only, so far)
- [ ] Fuzz targets and Miri: moved to the-wider-reach
- [x] SIMD past the baseline: the user asked for it (2026-11-01); the AVX-512 backend with a `QuotedPrintableCpu` token, as base64-codec has
- [ ] Ask the user about Q and DKIM modes: not asked this pass; moved to the-wider-reach
- [x] Re-bench (median of three criterion runs), numbers into README.md and benches/README.md, a log entry, the next handoff
