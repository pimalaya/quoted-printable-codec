---
cairn: log
change: the-installed-token
landed: 2026-11-02
---

# The installed token

Landed default codecs on the token the application installs: `QuotedPrintableCpu::default()` is `QuotedPrintableCpu::new(SimdCpu::current())`, the installed token or `SimdCpu::BASELINE`, and the derived `QuotedPrintableCodec::default()` takes it. An explicit token stays the `cpu` field, and `QuotedPrintableCodec::BASELINE`, new, is the deterministic codec on the build target's backend. The token is read once when the value is built, never by an encode or decode call.

`QuotedPrintableDecode::new` built its fields with `..Self::default()`, which would now have read the token for a codec it drops; it fills them itself. The release assembly is otherwise unchanged: only the panic line numbers of src/decode.rs moved.

simd-cpu is pinned at f77308a, the rev holding the install, in Cargo.toml and fuzz/Cargo.toml. tests/installed.rs, alone in its binary since the token is the process's, checks a default codec before and after an install; it fails on the old `const` default. Miri was not run: no unsafe code or kernel changed.

Capabilities moved: backends (MODIFIED: CPU detection is the caller's).
