---
cairn: tasks
change: the-wide-blocks
---

- [ ] Rebuild the measuring harnesses (perf in both builds, before/after, fuzz) as the proposal describes
- [ ] 64-lane blocks: the 1 MiB and 4 KiB text gains without losing a 64-byte row or binary encoding
- [ ] Decoding binary at 64 bytes: a clear lead over encodify in both builds (store side, spills)
- [ ] Size: under 506 lines, no row lost
- [ ] aarch64: build and measure (no `pmovmskb`, the masks may compile badly)
- [ ] Fuzz targets and Miri
- [ ] Ask the user about Q and DKIM modes, and about SIMD past the baseline (unsafe, a CPU token)
- [ ] Re-bench (median of three criterion runs), numbers into README.md and benches/README.md, a log entry, the next handoff
