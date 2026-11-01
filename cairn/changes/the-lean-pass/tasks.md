---
cairn: tasks
change: the-lean-pass
---

- [ ] Rebuild the measuring harnesses (perf in both builds, before/after, fuzz) as the proposal describes
- [ ] Size: under the 507 lines of the pass before, then lower, no row lost
- [ ] Decoding binary at 64 bytes: a clear lead over encodify in both builds and in criterion
- [ ] Encoding binary at 4 KiB: locate the 35 misses a KiB
- [ ] aarch64: build and measure (no `pmovmskb`, the masks may compile badly)
- [ ] Fuzz targets and Miri
- [ ] Reuse after `finish`: reset both coroutines fully, or specify that a finished one is dropped
- [ ] Ask the user about Q and DKIM modes, and about SIMD past the baseline (unsafe, a CPU token)
- [ ] Re-bench, numbers into benches/README.md, a log entry, the next handoff
