---
cairn: change
id: the-small-calls
status: landed
created: 2026-11-01
---

# The small calls

The handoff of the second session of 2026-11-01 (log 2026-11-01-the-speed-plan), written for a fresh context to take over the optimisation without its history: where things stand, how to measure without fooling yourself, where the time goes, what was tried, and what is left.

## Why

The user's goal, as for base64-codec (../base64-codec, its cairn has seven passes of history): beat encodify 1.0.2 in every section of benches/compare.rs with the most concise code, each pass better than the last in speed and in size, free hands, searching online for new methods where needed. encodify's qp module and simdutf8 are references only. This pass made the crate 2x to 20x faster than the scalar baseline; it leads encodify in every 1 MiB row and in ascii at 4 KiB, trails it in the six 64-byte rows, and in accented and binary at 4 KiB is ahead or behind depending on the method (criterion ahead, cycle counts behind by 5% to 21%), and its code grew from 422 to 507 lines (tests apart).

Settled this pass: no unsafe code and no CPU token so far (the compiler vectorizes the masks on any target); the decoder's blank cap slides (a line break deletes the last 76 blanks, keeps the older ones); every call may overwrite `dst` past what it reports written. Rules kept from before: lowercase hex digits decode but report unclean; a malformed escape keeps `=` (and its hex digit) and decodes what follows again; decoding never fails; names use `QuotedPrintable`, never `Qp`.

## Where things stand

The code: src/lib.rs holds `is_plain`, `is_blank`, `mask` (a 32-lane bitmask, two folds of 16 lanes, `#[inline(never)]`) and `window` (the next 64 bytes of the input, or a zero-padded copy when fewer remain). src/encode.rs `blocks` encodes 32 bytes a step: one mask of the non-plain bytes, events for line-final blanks, a walk over the events copying each literal run with one 32-byte copy and splitting it at column 75, escapes from a 16-byte digit table. src/decode.rs `tokens` alternates `block` (32 bytes, masks of `=` and of the plain bytes, a walk over the `=`, a scalar check of every other special byte, `Err` with the offset of the first byte it leaves) and token steps, byte by byte, for what blocks leave. src/edge.rs `Edge` carries the undecided bytes and queues the output with no room, running the core over them.

Cycles per byte of raw input (`perf stat`, core 13, one source and output offset pair), against encodify:

| workload | cycles | instructions | misses per KiB | encodify cycles | encodify instructions | encodify misses per KiB |
| --- | --- | --- | --- | --- | --- | --- |
| encode ascii, 64 B | 2.76 | 15.9 | 0 | 2.02 | 12.3 | 0 |
| encode ascii, 4 KiB | 1.38 | 9.5 | 0 | 1.96 | 11.2 | 0 |
| encode ascii, 1 MiB | 2.73 | 9.3 | 40 | 4.20 | 11.0 | 80 |
| decode ascii, 64 B | 2.70 | 14.8 | 0 | 1.64 | 10.2 | 0 |
| decode ascii, 4 KiB | 1.41 | 9.3 | 0 | 1.75 | 9.4 | 0 |
| decode ascii, 1 MiB | 2.54 | 9.2 | 39 | 3.68 | 9.2 | 73 |
| encode accented, 64 B | 5.72 | 36.2 | 0 | 4.29 | 30.6 | 0 |
| encode accented, 4 KiB | 3.79 | 27.5 | 1 | 4.24 | 28.3 | 0 |
| encode accented, 1 MiB | 6.49 | 26.8 | 91 | 10.77 | 27.3 | 226 |
| decode accented, 64 B | 5.79 | 30.8 | 0 | 4.41 | 26.1 | 0 |
| decode accented, 4 KiB | 4.13 | 22.4 | 0 | 3.92 | 23.3 | 0 |
| decode accented, 1 MiB | 5.82 | 21.8 | 82 | 9.55 | 22.6 | 212 |
| encode binary, 64 B | 9.00 | 62.9 | 0 | 7.34 | 48.7 | 0 |
| encode binary, 4 KiB | 7.43 | 47.9 | 38 | 6.79 | 46.5 | 0 |
| encode binary, 1 MiB | 8.07 | 47.5 | 60 | 21.54 | 46.3 | 480 |
| decode binary, 64 B | 9.01 | 50.5 | 0 | 5.94 | 39.4 | 0 |
| decode binary, 4 KiB | 7.30 | 43.7 | 1 | 6.04 | 39.9 | 0 |
| decode binary, 1 MiB | 9.25 | 43.3 | 77 | 18.51 | 39.7 | 464 |

Criterion (benches/README.md) puts the crate ahead in every 4 KiB row; the cycles above put encodify ahead decoding accented (5%) and binary (21%) and encoding binary (9%) at 4 KiB. The methods disagree there: measure both, averaged over offset pairs, before claiming those rows. Binary encoding mispredicts 38 times a KiB at 4 KiB where nothing else does; which branch is not measured yet (the soft break before an escape and the run split at column 75 are the candidates).



Criterion medians are in benches/README.md.

## How to measure

- Cycles, instructions and branch misses per byte, not time: `perf stat -x, -e cycles:u,instructions:u,branch-misses:u` (perf from `nix-shell -p linuxPackages.perf`) over a binary running one call N times, at N and 2N, the difference over N times the size, pinned with `taskset -c 13` (its sibling is 29; check both idle with `ps -eo psr,pcpu,comm`). Start fuzzers and builds pinned away (`taskset -c 2-11`).
- The harness this pass used, in a scratch crate depending on the tree by path and on encodify `=1.0.2`: `qpbench <qp|enc> <encode|decode> <ascii|accented|binary> <size> <iters> [src offset] [dst offset]`, the inputs generated as benches/compare.rs does, buffers at those offsets from a page boundary. base64-codec's handoff (the-store-queue) refines this: average over eight offset pairs, and build with `RUSTFLAGS='-C llvm-args=-align-all-functions=6 -C llvm-args=-align-loops=64'`; binary decoding moved by under 7% over eight pairs this pass.
- The 64-byte and 4 KiB rows count instructions: Zen 5 memorizes the branch history of an input repeated up to about 64 KiB, so no library mispredicts there. The 1 MiB rows count mispredictions (40 to 90 per KiB here, 70 to 480 for encodify). Read both before keeping a change.
- Correctness: a differential fuzzer in a scratch crate, depending on the tree and on a copy of the tree before the session (`git archive HEAD src Cargo.toml`, package renamed, `[[bench]]` dropped): random bodies over the bytes that matter (`=`, blanks, CR, LF, hex digits both cases, 0x00, 0x7F, 0xC3, 0xFF, runs of `x`, soft breaks, long blank runs), every encode mode and both line endings against the old one-shot calls, decoding raw and encoded bodies (skip bodies with more than 76 blanks in a row, where the cap now slides), and random chunks with 1 to 300 bytes of room against the new one-shot. It found two bugs this pass; run 300,000 bodies before keeping a change. In the crate, `every_split_gives_the_same_output` and `a_frozen_corpus_keeps_its_output` (src/lib.rs) hold the same checks smaller.
- Assembly: `cargo rustc --release --lib -- --emit asm`, with `#[inline(never)]` on the function looked at; `perf record -e cycles:u` and `perf annotate --stdio -s <function>` for the hot instructions. Samples pile on the instruction after a stall: a load from a stack buffer just written at other offsets is a store-forwarding stall.

## Where the time goes

- 64-byte calls: a 66-byte encoded input takes three blocks (32, 32 and 2 bytes), each paying two out-of-line `mask` calls, the `window` copy when fewer than 64 bytes remain (a `memcpy` per 16-byte chunk), the walk's setup and the trailing-blank count. Encoding pays the same plus `Edge::new` and the coroutine glue, since `encode` runs the coroutine.
- Binary at 4 KiB: 44 instructions a raw byte decoding against encodify's 40, 48 encoding against 46, with no mispredictions on either side: the walk pays per escape two `HEX` loads, the check, a 32-byte run copy even when the run is empty and the cursor updates decoding; the soft-break check, the digit lookups, a 4-byte store and the run split check encoding. encodify's scalar escape loops do less per escape.
- Masks: about 15 to 30 instructions each, out of line; inlined, SLP merges them into shuffles.

## Levers left

Ordered by expected gain, each to be measured before it is kept:

1. 64-byte calls: send tails shorter than a few bytes to the token steps instead of a block; fill `window` with fixed 16-byte copies (`as_chunks`) and one short copy; a one-shot `encode` that runs `blocks` directly and needs the coroutine only for the last block; 64-lane blocks (u64 masks, 64-byte run copies) so that a 64-byte input is one block.
2. Binary at 4 KiB: a tight inner loop over adjacent escapes (`=XX=XX`, the run copy skipped), in both directions; a dense per-lane path when a block holds more than about 12 events; the decode values of all 32 lanes computed as vectors (`(b & 0xf) + 9 * (b >> 6)` over the input at offsets 1 and 2) instead of per escape.
3. Size: `tokens`' escape and line-break arms repeat what `block` does; `Edge` could serve the one-shot calls too; the 85 lines over the baseline are mostly the decoder's block and the encoder's run split.
4. aarch64: never built (fenix's aarch64 rust-std and qemu in a throwaway shell, as base64-codec does). There is no `pmovmskb`: the masks may compile into a slow emulation, in which case a NEON `mask` (the `shrn` nibble mask, 4 bits a lane) is the lever, and with it `unsafe`.
5. SIMD past the baseline (SSSE3 `pshufb` compress and expand, AVX2): needs `unsafe` and a CPU token from the application, as base64-codec has; only if 1 and 2 leave encodify ahead.
6. Fuzz targets, Miri, and the Q and DKIM modes (ask the user first: they may belong to another crate).

## Traps

- Out-of-line `mask` is load-bearing: inlined, the block's masks become a tangle of shuffles 2x to 3x slower. Folds over 32 lanes do not vectorize at all.
- Store forwarding: a stack buffer written with copies of one size and read at other offsets stalls; blocks read the caller's input directly and write the decoder's output straight into `dst` when 64 bytes of room remain.
- The decoder's block folded into `tokens` runs out of registers (70% slower on escapes); it stays a function of its own.
- A block's lookahead: a decision on the last lane needs up to 2 bytes past it; the encoder cuts an undecided CR and blank off the end unless the input ends, the decoder sends them to the token steps.
- Soft-break tails: a CR or LF right after `=` belongs to the soft break, and a bare CR right after a `=\n` soft break does not (the fuzzer's first find).
- SSE2 intrinsics need `unsafe` with rustc 1.99 although SSE2 is x86-64's baseline.
