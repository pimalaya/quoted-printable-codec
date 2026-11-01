---
cairn: change
id: the-lean-pass
status: draft
created: 2026-11-01
---

# The lean pass

The handoff of the third session of 2026-11-01 (log 2026-11-01-the-small-calls), written for a fresh context to take over the optimisation without its history: where things stand, how to measure without fooling yourself, where the time goes, what was tried, and what is left.

## Why

The user's goal, as for base64-codec (../base64-codec, its cairn has the longer history): beat encodify 1.0.2 in every section of benches/compare.rs with the most concise code, each pass better than the last in speed and in size, free hands, searching online for new methods where needed (a search found no SIMD quoted-printable codec to learn from; yEnc SIMD decoders, such as node-yencode's, are the closest relatives). encodify's qp module and simdutf8 are references only. The crate now leads encodify in all 18 rows by cycle counts; the tightest row is decoding binary at 64 bytes (8%), and the code is 526 lines against 507 a pass ago (non-blank, non-comment lines before each `#[cfg(test)]`, after `cargo fmt`). This pass should above all win back lines without losing a row.

Settled so far: no unsafe code and no CPU token (the compiler vectorizes the masks on any target); the decoder's blank cap slides (a line break deletes the last 76 blanks, keeps the older ones); every call may overwrite `dst` past what it reports written, within the length it was given (the one-shot `decode` uses all of it); lowercase hex digits decode but report unclean; a malformed escape keeps `=` (and its hex digit) and decodes what follows again; decoding never fails; names use `QuotedPrintable`, never `Qp`.

## Where things stand

The code: src/lib.rs holds `is_plain`, `is_blank`, `mask` (a 32-lane bitmask, two fixed 16-lane folds inlined into it, the whole `#[inline(never)]`), `pad` (the input's last 64 bytes ending at offset 64 of a zeroed 128-byte buffer) and `window` (a block's 64 bytes, from the input or from the pad). src/decode.rs: `tokens` builds the pad, then alternates `blocks` (one call, blocks of 32 bytes over the caller's buffers, one ordered walk over the mask of non-plain bytes, the block committed up to the first byte it cannot decode) and token steps, the scalar rules byte by byte, for one token after a stop and for the last 8 bytes; `escape` reads a two-row `u16` digit table. src/encode.rs: `blocks` walks the mask of non-plain bytes, copying each literal run with one 32-byte copy split at column 75, escapes from a 256-entry `[[u8; 4]; 256]` table; the one-shot `encode` calls it directly, through a 268-byte spare for the last blocks only. src/edge.rs `Edge` carries the undecided bytes and queues the output with no room, for the coroutines.

Since a-codec-value (log 2026-11-01-a-codec-value), the one-shot calls are methods of `QuotedPrintableCodec` (src/codec.rs), `encode` and `decode` `#[inline]`, and the code is 531 lines against 526 before it: measure size against 531. A harness calls `QuotedPrintableCodec { eol, binary }.encode` and `.decode` on this tree, the free functions on `qpref` copies from before it.

Cycles per byte of raw input (`perf stat`, core 13, plain build, averaged over eight source and output offset pairs), with the pass before (HEAD of 2026-11-01, commit bb7e953) and encodify:

| workload | cycles | instructions | misses per KiB | cycles before | encodify cycles | encodify instructions | encodify misses per KiB |
| --- | --- | --- | --- | --- | --- | --- | --- |
| encode ascii, 64 B | 1.24 | 8.1 | 0 | 2.90 | 1.98 | 12.4 | 0 |
| encode ascii, 4 KiB | 1.08 | 7.4 | 0 | 1.38 | 1.97 | 11.2 | 0 |
| encode ascii, 1 MiB | 2.25 | 7.3 | 36 | 2.66 | 4.15 | 11.0 | 79 |
| encode accented, 64 B | 3.24 | 22.9 | 0 | 5.47 | 4.70 | 30.7 | 0 |
| encode accented, 4 KiB | 2.85 | 20.5 | 1 | 3.72 | 4.26 | 28.4 | 0 |
| encode accented, 1 MiB | 5.26 | 20.0 | 87 | 6.19 | 10.52 | 27.3 | 224 |
| encode binary, 64 B | 6.42 | 40.3 | 0 | 9.08 | 7.37 | 48.8 | 0 |
| encode binary, 4 KiB | 5.29 | 33.8 | 35 | 7.29 | 6.69 | 46.5 | 0 |
| encode binary, 1 MiB | 5.80 | 33.5 | 59 | 7.86 | 21.45 | 46.3 | 478 |
| decode ascii, 64 B | 1.49 | 8.9 | 0 | 2.63 | 1.70 | 10.4 | 0 |
| decode ascii, 4 KiB | 0.94 | 5.8 | 0 | 1.43 | 1.73 | 9.4 | 0 |
| decode ascii, 1 MiB | 2.32 | 5.8 | 41 | 2.59 | 3.71 | 9.2 | 74 |
| decode accented, 64 B | 3.53 | 18.5 | 0 | 5.88 | 4.30 | 26.2 | 0 |
| decode accented, 4 KiB | 2.66 | 14.1 | 0 | 3.63 | 3.89 | 23.3 | 0 |
| decode accented, 1 MiB | 4.83 | 13.8 | 83 | 5.82 | 9.48 | 22.6 | 212 |
| decode binary, 64 B | 5.52 | 31.3 | 2 | 9.65 | 5.96 | 39.6 | 0 |
| decode binary, 4 KiB | 4.92 | 27.5 | 0 | 7.44 | 6.05 | 39.9 | 0 |
| decode binary, 1 MiB | 6.89 | 27.3 | 78 | 9.35 | 18.45 | 39.7 | 464 |

Criterion medians are in benches/README.md.

## How to measure

- Cycles, instructions and branch misses per byte, not time: `perf stat -x, -e cycles:u,instructions:u,branch-misses:u` (perf from `nix-shell -p perf`) over a binary running one call N times, at N and 2N, the difference over N times the size, pinned with `taskset -c 13` (its sibling is 29; check both idle with `ps -eo psr,pcpu,comm`, as another process landing there once made a row 60% slower for a run). Start fuzzers and builds pinned away (`taskset -c 2-11`), and never build while criterion runs.
- The harness, in a scratch crate depending on the tree by path, on a copy of the tree before the session (`git archive HEAD src Cargo.toml`, package renamed `qpref`, `[[bench]]` and dev-dependencies dropped) and on encodify `=1.0.2`: `qpbench <qp|ref|enc> <encode|decode> <ascii|accented|binary> <size> <iters> [src offset] [dst offset]`, the inputs generated as benches/compare.rs does, buffers at those offsets from a page boundary; a script averages eight offset pairs (0,0 1,7 3,13 5,33 8,1 13,61 21,5 34,40). This session's copies are in /tmp/claude-1000/-home-soywod-code-pimalaya/48184fb3-127f-4cdd-b6fa-9de4dbaa4c81/scratchpad (qpbench, qpref, qpdiff, pstat.sh, quick.sh, all.sh) while /tmp lasts.
- Build the harness twice, plain and with `RUSTFLAGS='-C llvm-args=-align-all-functions=6 -C llvm-args=-align-loops=64'` into another target directory, and read both: code layout alone moves a row by up to 20%. A change counts when it wins in both builds, or wins clearly in instructions without losing cycles in either.
- The 64-byte and 4 KiB rows count instructions: Zen 5 memorizes the branch history of an input repeated up to about 64 KiB, so no library mispredicts there (binary encoding at 4 KiB, 35 a KiB, is the exception). The 1 MiB rows count mispredictions. Read both before keeping a change.
- Correctness: a differential fuzzer in the scratch crate, against `qpref`: random bodies over the bytes that matter (`=`, blanks, CR, LF, hex digits both cases, 0x00, 0x7F, 0xC3, 0xFF, runs of `x`, soft breaks, long blank runs), every encode mode and both line endings against the old one-shot calls, decoding raw and encoded bodies, and random chunks with 1 to 300 bytes of room against the new one-shot; its release profile keeps debug assertions and overflow checks. Run 300,000 bodies before keeping a change, and `cargo test` in debug too. In the crate, `every_split_gives_the_same_output` and `a_frozen_corpus_keeps_its_output` (src/lib.rs) hold the same checks smaller.
- criterion (`cargo bench --features alloc --bench compare`, pinned) is what benches/README.md publishes; its 64-byte rows swing by about 6% run to run, so a row within that of encodify needs the `perf` table too.
- Assembly: `cargo rustc --release --lib -- --emit asm -C llvm-args=-x86-asm-syntax=intel`, functions found by their v0-mangled names (`6decode6blocks`); `perf record -e cycles:u` and `perf annotate --stdio -s <function>`. Samples pile on the instruction after a stall; `instructions:u` sampling skids too far to place anything.

## Where the time goes

- Decoding binary: about 25 instructions an escape (load, two table loads, OR, a 32-byte run copy, a byte store, cursor updates, the walk), and about 45 of scaffolding a block plus the 29-instruction mask; IPC is about 5.5, so instruction count rules at 4 KiB. At 64 bytes the 146-byte input runs five blocks, two through the pad, and the token steps take the last few bytes.
- Encoding binary: the escape path (soft-break check, table word, cursor updates) and the run split at column 75; the 35 misses a KiB at 4 KiB are not located (the soft break before an escape and the run split are the candidates; a branch-free soft break was slower).
- 64-byte calls: the pad (a 128-byte buffer zeroed and 64 bytes copied), the spare in the one-shot encode (a 268-byte `memset` and a `memmove` when the last block lacks 134 bytes of room, as in binary), the call and setup of `blocks`.

## Levers left

Ordered by expected gain, each to be measured before it is kept:

1. Size: the 19 lines over the pass before are the escape table (6 more than computing the digits), the two-row digit table and `escape` (4 more than `HEX`), the one-shot encode's spare (7) and the block loops' scaffolding. Candidates: the encoder's spare moved into `blocks` per block, as the decoder's (then `encode` is three lines; the edge's queue covers the coroutine), the shared output construction in decode.rs, the token steps' escape arm (dead when `blocks` runs to the end, which costs the 64-byte rows), and anything both `blocks` repeat.
2. Decoding binary at 64 bytes, 8% ahead: fewer blocks for a short input (64-lane blocks, u64 masks, would halve the scaffolding but need 64-byte run copies), a cheaper pad (zero only its second half), the token steps' share of the tail.
3. Encoding binary at 4 KiB: find the 35 misses a KiB (`perf record -e branch-misses:u`).
4. aarch64: never built (fenix's aarch64 rust-std and qemu in a throwaway shell, as base64-codec does). There is no `pmovmskb`: the masks may compile into a slow emulation, in which case a NEON `mask` (the `shrn` nibble mask, 4 bits a lane) is the lever, and with it `unsafe`.
5. SIMD past the baseline (SSSE3 `pshufb` compress and expand, AVX2, as yEnc decoders do): needs `unsafe` and a CPU token from the application, as base64-codec has; only if the user wants it, since every row already leads.
6. Fuzz targets, Miri, and the Q and DKIM modes (ask the user first: they may belong to another crate).
7. Reuse after `finish` is unspecified: the edge resets itself, but the encoder keeps its column (since this pass, the one before `finish`) and the decoder its dirty flag. Reset both coroutines fully, or say in the spec that a finished coroutine is dropped.

## Traps

- `mask` must stay out of line and its halves inlined: inlined into a block, the masks merge into slow shuffles; over slices or with `(0..16).fold`, the halves go out of line with bounds checks. Folds over 32 lanes do not vectorize at all.
- Store forwarding: a buffer written with stores of one size and read with loads of another stalls. Build the pad before the blocks run, with whole 64-byte copies; a `u128` tail chunk is two 8-byte stores.
- The escape's validity must stay a branch: branch-free (`r = at + 2 + escape`), the read cursor waits on two chained table loads and the walk serializes (binary decoding 7.4 against 5.0 cycles a byte).
- `count_ones` and `trailing_zeros` are no single instruction on baseline x86-64 (no POPCNT, BMI); `trailing_zeros` compiles to `rep bsf`, fine, `count_ones` to a dozen instructions.
- A block needs 64 bytes of input (32 lanes, 2 read ahead, runs copied 32 bytes at a time) and of output (the decoder) or 134 (the encoder); the edge's queue is sized to the encoder's 134, so any room estimate must stay within it or the coroutine stops progressing.
- Soft-break tails: a CR or LF right after `=` belongs to the soft break, and a bare CR right after a `=\n` soft break does not; a dropped bare CR does not end a run of blanks, so a block may not judge a line break's blanks from the byte before it alone (the decoder carries `blanks` across blocks).
- SSE2 intrinsics need `unsafe` with rustc 1.99 although SSE2 is x86-64's baseline.
