---
cairn: tasks
change: the-installed-token
---

- [x] the-q-encoding landed; simd-cpu's an-installed-token committed and pushed (f77308a), its rev pinned in Cargo.toml and fuzz/Cargo.toml
- [x] `QuotedPrintableCpu::default()` on `SimdCpu::current()`, `QuotedPrintableCodec::BASELINE` added; `QuotedPrintableDecode::new` fills its fields itself instead of `..Self::default()`, which would have read the token for a codec it drops
- [x] tests/installed.rs: a default codec runs the baseline backend before an install and the installed token's after, and a codec built before the install keeps its backend
- [x] Release assembly diffed against HEAD: no instruction changed, only panic line numbers of src/decode.rs; the coroutine constructors no longer touch `Default`
- [x] Crate header, `QuotedPrintableCpu` and `QuotedPrintableCodec` docs, README (the repository has no CHANGELOG.md)
- [x] Tests, clippy (aarch64 too), docs, the fuzz crate; Miri skipped, no unsafe code or kernel touched
- [x] Spec folded, log entry, change archived
