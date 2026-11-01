---
cairn: tasks
change: the-small-calls
---

- [x] Rebuild the measuring harnesses (perf, before/after, fuzz) as the proposal describes
- [x] 64-byte calls: fewer blocks or cheaper ones, ahead of encodify in the six rows
- [x] Binary at 4 KiB: cheaper escapes, ahead of encodify encoding and decoding
- [ ] Size: back under the 422 code lines of the scalar baseline, or a reason why not: 526 lines against 507 before the pass, reasons in log 2026-11-01-the-small-calls; moved to the-lean-pass
- [ ] aarch64: build and measure: moved to the-lean-pass
- [ ] Fuzz targets and Miri: moved to the-lean-pass
- [ ] Ask the user about Q and DKIM modes: moved to the-lean-pass
- [x] Re-bench, numbers into benches/README.md, a log entry, the next handoff
