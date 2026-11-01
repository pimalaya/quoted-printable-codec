---
cairn: tasks
change: the-column-chain
---

- [ ] Rebuild the measuring harnesses (perf in both builds, both backends, the tree against the tree before, fuzz per backend) as the proposal describes
- [ ] Text encoding: fewer live values and a shorter column chain, toward binary's 0.46 cycles a byte, no row lost
- [ ] AVX2 backend: decoding and encoding ahead of the portable rows, its lines weighed against the gain
- [ ] NEON backend, and an aarch64 measure of the portable one
- [ ] Decoding: fewer compares and mask moves a block
- [ ] The 64-byte rows: a clear lead over this pass in both builds, over several runs
- [ ] Size: under 786 lines, no row lost
- [ ] Miri on the portable backend, fuzz targets
- [ ] Ask the user about Q and DKIM modes
- [ ] Re-bench (median of three criterion runs), numbers into README.md and benches/README.md, a log entry, the next handoff
