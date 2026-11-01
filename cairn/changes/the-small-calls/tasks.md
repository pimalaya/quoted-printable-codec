---
cairn: tasks
change: the-small-calls
---

- [ ] Rebuild the measuring harnesses (perf, before/after, fuzz) as the proposal describes
- [ ] 64-byte calls: fewer blocks or cheaper ones, ahead of encodify in the six rows
- [ ] Binary at 4 KiB: cheaper escapes, ahead of encodify encoding and decoding
- [ ] Size: back under the 422 code lines of the scalar baseline, or a reason why not
- [ ] aarch64: build and measure (no `pmovmskb`, the masks may compile badly)
- [ ] Fuzz targets and Miri
- [ ] Ask the user about Q and DKIM modes
- [ ] Re-bench, numbers into benches/README.md, a log entry, the next handoff
