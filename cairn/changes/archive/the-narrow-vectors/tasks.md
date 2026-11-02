---
cairn: tasks
change: the-narrow-vectors
---

- [x] AVX2 backend, decoding and encoding
- [x] NEON backend, decoding and encoding (chunks of 32 bytes after the finish regression)
- [x] Token, dispatch, `available()` listing every backend
- [x] Tests on x86_64 (debug, release, native, no default features) and on aarch64 under qemu (debug, release)
- [x] Miri on the portable backend, the AVX2 kernels and, for aarch64, the NEON ones
- [x] Fuzz targets, run on every x86_64 backend
- [x] CI jobs, the nightly shell, README and docs (the CI jobs not run: nothing pushed)
- [x] Spec folded, log entry, the-column-chain updated
