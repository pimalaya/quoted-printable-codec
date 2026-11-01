---
cairn: change
id: the-speed-plan
status: draft
created: 2026-11-01
---

# The speed plan

The handoff of the session of 2026-11-01 (logs 2026-11-01-a-quoted-printable-codec and 2026-11-01-a-first-bench), written for a fresh context to take over without its history: what the crate is, where it stands, how to measure, and what to do next.

## Why

The user's goal, by analogy with base64-codec (../base64-codec, its cairn has the whole optimisation history): a quoted-printable codec that is correct first, then beats encodify 1.0.2 in every section of benches/compare.rs with the most concise code. base64-codec is the model for the structure (modules, coroutine API, CPU token, driver stamped into backends, tests, spec); encodify's qp module and simdutf8 are implementation references only. The first session was told not to think about speed: it built a correct scalar baseline and measured it.

## Where things stand

The crate: src/lib.rs (docs, `#![no_std]`, `#![forbid(unsafe_code)]`), src/encode.rs (`QuotedPrintableEncode`, `QuotedPrintableEncodeOptions { eol, binary }`, `QuotedPrintableEol`, `encode`, `encode_vec`, `encoded_len_max`), src/decode.rs (`QuotedPrintableDecode`, `decode`, `decode_vec`, `decoded_len_max`), src/queue.rs (a fixed-size output queue). Names use the full `QuotedPrintable` prefix, never `Qp` (the user's rule). Features: `alloc` only; `std` was dropped since nothing needs it yet (it comes back with a `QuotedPrintableCpu::detect`).

How it works: both coroutines consume one input byte per step. The encoder carries a pending space or tab and, in text mode, a pending CR, until the next byte tells whether a line ends there; the decoder is a six-state machine (`Text`, `Cr`, `Equals`, `Hex`, `EqualsBlanks`, `EqualsCr`) carrying up to 76 trailing blanks. Each step pushes its output into the queue, `resume` drains the queue into `dst` before taking the next byte, so one byte of room always progresses and the one-shot functions are the coroutine run once. The rules are in cairn/spec (encoding, decoding, calls); the tests in each module check them, the split invariance, the length bounds and the encode/decode round trip.

Rules that differ from encodify on purpose (keep them, or ask the user before changing): lowercase hex digits decode but report unclean; a malformed escape keeps `=` (and its hex digit) and decodes the bytes after it again as data, where encodify keeps `=` and the next byte and skips both; decoding never fails and returns a `clean` flag, as base64-codec does, where encodify's strict engines return errors. The encoder soft-breaks before a token that would pass 75 characters, even when a hard break follows and 76 would fit.

The numbers (benches/README.md, criterion medians, one core): 0.07 to 0.23 GiB/s, last in every row. encodify is 3x to 16x ahead on text and 1.8x to 10x on binary, quoted_printable 0.5.2 1.05x to 4x (while allocating), mail-parser 0.11.9 3x to 10x on decoding. The cost is flat per byte: a state-machine step, a queue push and a queue drain per input byte.

## How to measure

- The repository is a git repo now: `nix develop` refuses untracked files, so use `nix develop path:$PWD --command ...` until everything is tracked.
- `taskset -c 13 cargo bench --features alloc --bench compare`, all groups in one run; the machine is shared and its clock moves, so compare within one run, and use cycles per call (`perf stat`) for differences of a few percent, as base64-codec's proposal the-store-queue describes.
- Every library slows at 1 MiB in the first run (encodify's ascii decode 2.6 to 1.2 GiB/s): not yet known whether that is the machine or the cache.
- The bench asserts every decoder gives back the raw bytes before timing; keep that when adding inputs.

## What to do, in order

1. Scalar fast paths, still safe code: copy runs of literal bytes (encoding: `!` to `~` but `=`, up to the line's room; decoding: anything but `=`, CR, LF, blanks) straight from `src` into `dst`, and write into `dst` directly when it has room for a whole step, the queue only for the last few bytes of a call. Escapes in a run (`=XX=XX...`) decoded and encoded a run at a time. This alone should reach the quoted_printable crate and close part of the gap to encodify's scalar `Swar` kernel (word-at-a-time, src/qp/kernel/scalar.rs).
2. A byte class table like encodify's src/qp/tables.rs `CLASS` (PLAIN, BLANK, ESCAPE, CR, LF) and an escape table (`ESCAPE_WORDS`) to replace the matches.
3. Then the base64-codec structure for speed: a `QuotedPrintableCpu` token (application-detected, `BASELINE`), backends stamped from one driver macro, SIMD kernels after encodify's src/qp/kernel/x86.rs and neon.rs (`copy_text`, `copy_plain`, `decode_escapes`, `encode_escapes`: find the stops in 16 or 32 bytes by compare and movemask, copy the run), simdutf8 0.1.5 for the driver shape. That brings `unsafe`, so `#![forbid(unsafe_code)]` becomes base64-codec's `#![deny(unsafe_op_in_unsafe_fn, clippy::undocumented_unsafe_blocks)]`.
4. Every backend compared to the scalar reference byte for byte, as base64-codec's tests do (`cpu::available()`), plus a frozen-reference test, fuzz targets and Miri, before a number counts.

## References

- encodify 1.0.2: ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/encodify-1.0.2/src/qp (also unpacked at /tmp/encodify, which may be gone): encode.rs `Lines` and `Cursor::steps` for the encode loop, decode.rs `Body::run` for the decode loop, kernel/ for the kernels.
- simdutf8 0.1.5: ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/simdutf8-0.1.5 (also /tmp/simdutf8): src/implementation/ for the per-target modules and the algorithm macro.
- base64-codec: src/driver.rs, src/cpu.rs, its cairn log for what paid off and what did not.

## Out of scope so far

RFC 2047 Q encoding and DKIM quoted-printable (encodify's `Q_TEXT`, `Q_PHRASE`, `DKIM`), an exact `encoded_len`, ATTRIBUTION.md (owed once code follows encodify or simdutf8 closely), a README and a CHANGELOG. Ask the user before adding the Q and DKIM modes: they may belong to another crate.
