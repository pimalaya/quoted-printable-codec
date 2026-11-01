---
cairn: tasks
change: the-wider-reach
---

- [x] The shared token: `simd_cpu::SimdCpu` in place of `QuotedPrintableCpu`, as base64-codec did (the change a-shared-cpu-token)
- [x] Rebuild the measuring harnesses (perf in both builds, both backends, before/after, fuzz per backend) as the proposal describes
- [ ] AVX2 backend: decoding and encoding ahead of the portable rows, its lines weighed against the gain: not started; this pass went to the AVX-512 kernels and the size; moved to the-column-chain
- [ ] NEON backend, and an aarch64 build and measure of the portable one: not started; the aarch64 suite passes under qemu; moved to the-column-chain
- [ ] The 64-byte ASCII rows: a clear lead over the pass before in both builds: level, within the noise of these rows (encoding ASCII at 64 bytes still runs the portable walk; a call costs about 350 instructions before its second group in either backend); moved to the-column-chain
- [ ] Sparse text encoding with `vpexpandb`: not tried; the output-space masks it needs cost what the spread does (log 2026-11-01-the-wider-reach), and text encoding went from 1.0 to 0.78 cycles a byte by other means; moved to the-column-chain
- [x] Size: under 820 lines, no row lost: 786 lines, from 805 after the token change, every row ahead or level with the pass before
- [ ] Miri on the portable backend, fuzz targets: not done; moved to the-column-chain
- [ ] Ask the user about Q and DKIM modes: not asked; moved to the-column-chain
- [x] Re-bench (median of three criterion runs), numbers into README.md and benches/README.md, a log entry, the next handoff
