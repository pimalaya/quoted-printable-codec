---
cairn: tasks
change: the-wider-reach
---

- [x] The shared token: `simd_cpu::SimdCpu` in place of `QuotedPrintableCpu`, as base64-codec did (the change a-shared-cpu-token)
- [ ] Rebuild the measuring harnesses (perf in both builds, both backends, before/after, fuzz per backend) as the proposal describes
- [ ] AVX2 backend: decoding and encoding ahead of the portable rows, its lines weighed against the gain
- [ ] NEON backend, and an aarch64 build and measure of the portable one
- [ ] The 64-byte ASCII rows: a clear lead over the pass before in both builds
- [ ] Sparse text encoding with `vpexpandb`: fewer instructions a byte than the three-lane spread on ASCII, no row lost
- [ ] Size: under 820 lines, no row lost
- [ ] Miri on the portable backend, fuzz targets
- [ ] Ask the user about Q and DKIM modes
- [ ] Re-bench (median of three criterion runs), numbers into README.md and benches/README.md, a log entry, the next handoff
