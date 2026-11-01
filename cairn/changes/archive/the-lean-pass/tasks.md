---
cairn: tasks
change: the-lean-pass
---

- [x] Rebuild the measuring harnesses (perf in both builds, before/after, fuzz) as the proposal describes
- [x] Size: under the 507 lines of the pass before, then lower, no row lost: 506 lines against 531 at the start of the pass
- [ ] Decoding binary at 64 bytes: a clear lead over encodify in both builds and in criterion: not reached, level with encodify by cycle counts (ahead in the criterion median); the decoder is store-bound, reasons in log 2026-11-01-the-lean-pass; moved to the-wide-blocks
- [x] Encoding binary at 4 KiB: locate the 35 misses a KiB: the mode tested on every event (and its spilled flag); a const-generic walk per mode removed them
- [ ] aarch64: build and measure: moved to the-wide-blocks
- [ ] Fuzz targets and Miri: moved to the-wide-blocks
- [x] Reuse after `finish`: a successful `finish` resets both coroutines
- [ ] Ask the user about Q and DKIM modes, and about SIMD past the baseline: moved to the-wide-blocks
- [x] Re-bench, numbers into benches/README.md, a log entry, the next handoff
