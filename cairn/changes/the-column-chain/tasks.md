---
cairn: tasks
change: the-column-chain
---

- [ ] Rebuild the measuring harnesses (perf in both builds, all backends, the tree against the tree before, fuzz per backend) as the proposal describes
- [ ] Text encoding: fewer live values and a shorter column chain, toward binary's 0.46 cycles a byte, no row lost
- [ ] AVX2 backend (accuracy first since the-narrow-vectors): measured, then ahead of the portable rows, its lines weighed against the gain
- [ ] NEON backend (same): measured on aarch64, with the portable one
- [ ] Decoding: fewer compares and mask moves a block
- [ ] The 64-byte rows: a clear lead over this pass in both builds, over several runs
- [ ] Size: under 786 lines before the narrow vectors, no row lost
- [x] Miri on the portable backend, fuzz targets (the-narrow-vectors, 2026-11-02)
- [x] Ask the user about Q and DKIM modes (2026-11-02: Q landed in the-q-encoding, DKIM left out)
- [ ] Re-bench (median of three criterion runs), numbers into README.md and benches/README.md, a log entry, the next handoff
