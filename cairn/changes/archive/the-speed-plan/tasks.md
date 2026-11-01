---
cairn: tasks
change: the-speed-plan
---

- [x] Literal runs copied straight from `src` to `dst`, encoding and decoding, queue only at a call's edge
- [x] Escape runs encoded and decoded a run at a time (each escape an event of a 32-byte block's mask walk)
- [x] Byte class and escape tables (a 256-byte `HEX` decode table, a 16-byte digit table; byte classes as masks instead)
- [x] Re-bench, numbers into benches/README.md and a log entry
- [ ] `QuotedPrintableCpu` token, driver macro, scalar backend on it: dropped, safe code the compiler vectorizes needs no token
- [ ] SSSE3, AVX2 and NEON kernels after encodify's: dropped for this pass, see the-small-calls
- [x] Frozen-reference test (a corpus hash matched by the old implementation); fuzz targets, Miri and ATTRIBUTION.md moved to the-small-calls
- [ ] Ask the user about Q and DKIM modes: moved to the-small-calls
