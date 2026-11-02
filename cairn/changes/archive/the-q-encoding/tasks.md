---
cairn: tasks
change: the-q-encoding
---

- [x] `QuotedPrintableMode` in place of `binary`, through the code, tests, bench and fuzz targets
- [x] Q encoding and decoding, one-shot and resumed, on every backend
- [x] `encoded_len`, exact in every mode
- [x] Tests: mime's cases, every byte round trip, splits and room, the clean flag; mime's own functions matched on 1,000,000 random inputs
- [x] Fold the single-call helpers of the AVX2 and NEON decoders back into them
- [x] Checks: x86_64 and aarch64 suites, Miri, fuzz targets, clippy, docs
- [x] Spec folded, log entry, README
