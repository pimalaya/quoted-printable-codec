---
cairn: change
id: the-installed-token
status: landed
created: 2026-11-02
---

# The installed token

Follows the-q-encoding, in flight in another session on 2026-11-02: this change starts once it lands, on the codec as it leaves it (`QuotedPrintableCodec { cpu, eol, mode }`).

## Why

simd-cpu gained a process-wide token on 2026-11-02 (its change an-installed-token): the application calls `SimdCpu::install_detected()` once, first thing in `main`, and libraries read `SimdCpu::current()`, the installed token or `SimdCpu::BASELINE`. Today a `QuotedPrintableCodec` runs SIMD only when every layer between the application and the codec hands a `SimdCpu` down; mime-rs grew a `cpu` field on its options and on every entity for that alone. A codec built with its defaults should run the installed token instead.

## What

Settled with the user on 2026-11-02: an explicit token wins, and without one the codec takes the installed token, or the baseline when none is installed, with no error and no log.

- `QuotedPrintableCpu::default()` returns `QuotedPrintableCpu::new(SimdCpu::current())` instead of the baseline backend; `QuotedPrintableCodec::default()`, derived, takes it.
- An explicit token stays a struct field: `QuotedPrintableCodec { cpu: QuotedPrintableCpu::new(token), ..QuotedPrintableCodec::default() }`. The codec value holds the resolved backend, never an `Option`, so the token is read once, when the value is built, and no encode or decode call touches the static: per-call resolution would add an atomic load and a backend match to every call, and lose the backend being known to the compiler on small inputs. An install after a codec value was built does not reach that value.
- `QuotedPrintableCpu::new` stays `const`; a `QuotedPrintableCodec::BASELINE` const joins it, as base64-codec has, for consts and for tests that need the build target's backend, `default()` no longer being deterministic.
- The coroutines' own state keeps its defaults: `QuotedPrintableEncode::new` and `QuotedPrintableDecode::new` take the codec they are given and read no token.
- The crate header, the docs of `QuotedPrintableCpu` and `QuotedPrintableCodec`, and the README say the application installs the token once and libraries take the defaults, or pass a token they were given.
- simd-cpu's pin moves to the rev that holds `install`, in Cargo.toml (dependency and dev-dependency) and fuzz/Cargo.toml, the same rev as base64-codec and mime-rs, so one static serves them all.

## Risks

- `Default` now calls a run-time function: a run-time `new` inside `Default` once pushed this crate's decoder constructor past rustc's cross-crate inline threshold, which is why it reads `const { Self::new(SimdCpu::BASELINE) }` today. The release assembly of the coroutine constructors and the one-shot calls is diffed function by function against HEAD, and the bench's small rows compared, as the-column-chain's method says.
- Tests and examples that build `QuotedPrintableCodec::default()` would follow whatever token a test installed: the install is tested in its own integration test binary, tests/installed.rs, so no unit test sees an installed token, and the frozen corpus keeps its explicit tokens.
