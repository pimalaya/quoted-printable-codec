---
cairn: change
id: the-wide-blocks
status: landed
created: 2026-11-01
---

# The wide blocks

The handoff of the fourth session of 2026-11-01 (log 2026-11-01-the-lean-pass), written for a fresh context to take over the optimisation without its history: where things stand, how to measure without fooling yourself, where the time goes, what was tried, and what is left.

## Why

The user's goal, as for base64-codec (../base64-codec): beat encodify 1.0.2 in every section of benches/compare.rs with the most concise code, each pass better than the last in speed and in size, free hands, searching online for new methods where needed (no SIMD quoted-printable codec exists to learn from; yEnc SIMD decoders, such as node-yencode's, are the closest relatives). encodify's qp module and simdutf8 are references only. The crate leads encodify in 17 of the 18 rows by cycle counts, by 1.06x (decoding binary at 4 KiB, aligned build) to 4.9x; decoding binary at 64 bytes is level with it (averaged over three runs, 5.96 and 6.04 cycles a byte in the plain and aligned builds against 6.04 and 6.02), and decoding at 64 bytes moved by no more than its noise this pass (accented reads 10% behind the pass before in the aligned build, level in the plain one). The code is 506 lines against 531 a pass ago (non-blank, non-comment lines before each `#[cfg(test)]` in src/*.rs, after `cargo fmt`): measure size against 506.

Settled so far: no unsafe code and no CPU token (the compiler vectorizes the masks on any target); the decoder's blank cap slides (a line break deletes the last 76 blanks, keeps the older ones); every call may overwrite `dst` past what it reports written, within the length it was given (the one-shot `decode` uses all of it); `encoded_len_max` includes the 160 bytes an encoding block may touch past its output, so the one-shot `encode` needs no spare; a successful `finish` resets a coroutine; lowercase hex digits decode but report unclean; a malformed escape keeps `=` (and its hex digit) and decodes what follows again; decoding never fails; names use `QuotedPrintable`, never `Qp`.

## Where things stand

The code: src/lib.rs holds `is_plain`, `is_blank`, `mask` (a 32-lane bitmask, two fixed 16-lane folds inlined into it, the whole `#[inline(never)]`), `pad` (the input's last 64 bytes ending at offset 64 of a zeroed 128-byte buffer) and `window` (a block's 64 bytes, from the input or from the pad). src/encode.rs: `blocks` dispatches on the mode with a `match` to `walk::<TEXT>`, the loop compiled once per mode; a block masks the non-plain bytes into a u64 with a sentinel bit at its end, adds the blanks before a line break, and walks the bits keeping `off` (output offset minus input offset, biased by 128 so that dropped CRs never take it negative and `& 127` erases the bias) and `lim` (the output offset where the line is full); one check an event, `n + 3 > lim`, leads to the rare path that breaks the line in the run or before the escape; runs are copied 32 bytes at a time, escapes are 4-byte words from `ESCAPES`; the sentinel's event writes an escape past the output; all indices are masked into a 160-byte output window and the 64-byte input window, so there are no bounds checks. The one-shot `encode` is one `blocks` call. src/decode.rs: `tokens` builds the pad, then alternates `blocks` (blocks of 32 bytes over the caller's buffers to the input's end, an ordered walk with `r` and a biased `off`, the block committed up to the first byte it cannot decode) and one token step for what a block stops on (a malformed `=`, a bare CR, a line break after blanks, the end); `escape` reads a two-row `u16` digit table. src/edge.rs `Edge` carries the undecided bytes and queues the output with no room, for the coroutines.

Cycles per byte of raw input (`perf stat`, core 13, averaged over eight source and output offset pairs, plain build then aligned build), with the pass before (commit e3f7726) and encodify, both builds alike for them within a few percent:

| workload | cycles | instructions | misses per KiB | cycles before | encodify cycles | encodify instructions | encodify misses per KiB |
| --- | --- | --- | --- | --- | --- | --- | --- |
| encode ascii, 64 B | 1.20 / 1.18 | 7.0 | 0 | 1.19 / 1.16 | 1.97 / 1.96 | 12.4 | 0 |
| encode ascii, 4 KiB | 1.10 / 1.10 | 6.5 | 0 | 1.13 / 1.10 | 1.96 / 1.96 | 11.2 | 0 |
| encode ascii, 1 MiB | 2.16 / 2.17 | 6.4 | 33 | 2.25 / 2.21 | 4.16 / 4.18 | 11.0 | 80 |
| encode accented, 64 B | 3.02 / 3.06 | 17.7 | 0 | 3.23 / 3.23 | 4.74 / 4.67 | 30.8 | 0 |
| encode accented, 4 KiB | 2.58 / 2.58 | 15.5 | 0 | 2.87 / 2.82 | 4.24 / 4.24 | 28.4 | 0 |
| encode accented, 1 MiB | 4.13 / 4.15 | 15.2 | 59 | 5.33 / 5.10 | 10.54 / 10.44 | 27.3 | 225 |
| encode binary, 64 B | 3.77 / 3.56 | 21.5 | 0 | 6.36 / 5.32 | 7.26 / 7.33 | 48.8 | 0 |
| encode binary, 4 KiB | 3.20 / 3.21 | 18.6 | 0 | 5.61 / 5.18 | 6.72 / 6.89 | 46.5 | 0 |
| encode binary, 1 MiB | 4.37 / 4.64 | 18.5 | 51 | 6.17 / 5.66 | 21.45 / 21.53 | 46.3 | 478 |
| decode ascii, 64 B | 1.24 / 1.24 | 8.2 | 0 | 1.38 / 1.35 | 1.70 / 1.76 | 10.4 | 0 |
| decode ascii, 4 KiB | 0.91 / 0.91 | 5.6 | 0 | 0.94 / 0.91 | 1.73 / 1.72 | 9.4 | 0 |
| decode ascii, 1 MiB | 2.33 / 2.30 | 5.6 | 41 | 2.27 / 2.31 | 3.71 / 3.91 | 9.2 | 74 |
| decode accented, 64 B | 3.40 / 3.64 | 17.9 | 0 | 3.41 / 3.29 | 4.32 / 4.34 | 26.2 | 0 |
| decode accented, 4 KiB | 2.66 / 2.68 | 13.4 | 0 | 2.65 / 2.65 | 3.88 / 3.92 | 23.3 | 0 |
| decode accented, 1 MiB | 4.81 / 4.72 | 13.1 | 83 | 4.75 / 4.73 | 9.44 / 9.75 | 22.6 | 212 |
| decode binary, 64 B | 5.96 / 6.04 | 29.2 | 1 | 5.64 / 5.96 | 6.04 / 6.02 | 39.5 | 0 |
| decode binary, 4 KiB | 4.93 / 5.05 | 25.9 | 0 | 5.01 / 4.99 | 6.05 / 5.34 | 39.9 | 0 |
| decode binary, 1 MiB | 6.96 / 6.80 | 25.7 | 78 | 6.77 / 6.88 | 18.46 / 18.82 | 39.7 | 464 |

Criterion medians are in README.md and benches/README.md.

## How to measure

- Cycles, instructions and branch misses per byte, not time: `perf stat -x, -e cycles:u,instructions:u,branch-misses:u` (perf from `nix-shell -p perf`) over a binary running one call N times, at N and 2N, the difference over N times the size, pinned with `taskset -c 13` (its sibling is 29; check both idle with `ps -eo psr,pcpu,comm`). Start fuzzers and builds pinned away (`taskset -c 2-11`), and never build or run perf while criterion runs.
- The harness, in a scratch crate depending on the tree by path, on a copy of the tree before the session (`git archive HEAD src Cargo.toml`, package renamed `qpref`, `[[bench]]` and dev-dependencies dropped) and on encodify `=1.0.2`: `qpbench <qp|ref|enc> <encode|decode> <ascii|accented|binary> <size> <iters> [src offset] [dst offset]`, inputs generated as benches/compare.rs does, buffers at those offsets from a page boundary (`SMALL=1` gives decoding criterion's output length, `ADJ=k` small adjacent heap buffers as criterion has); scripts average eight offset pairs (0,0 1,7 3,13 5,33 8,1 13,61 21,5 34,40). This session's copies are in /tmp/claude-1000/-home-soywod-code-pimalaya/bd286a0e-7642-4de8-b024-36e2b17beb4e/scratchpad while /tmp lasts: build.sh (both bench builds and the fuzzer), pstat.sh and pstat-al.sh (one row, plain or aligned), quick.sh (rebuilds, then rows), all.sh and allal.sh (full tables, no rebuild), loc.sh (the line count), crit.pl and readme_tables.pl (criterion logs to the README tables, median of the runs given), v/ (every variant tried, v/s64 the 64-lane blocks).
- Build the harness twice, plain and with `RUSTFLAGS='-C llvm-args=-align-all-functions=6 -C llvm-args=-align-loops=64'` into another target directory, and read both after every edit, cold-path edits included: layout alone moves a row by up to 65% (log 2026-11-01-the-lean-pass). A change counts when it wins in both builds, or wins clearly in instructions without losing cycles in either.
- The 64-byte and 4 KiB rows count instructions: Zen 5 memorizes the branch history of an input repeated up to about 64 KiB. The 1 MiB rows count mispredictions. When cycles stop following instructions, `perf stat -M PipelineL1,PipelineL2` and `-e ls_stlf:u,ls_dispatch.store_dispatch:u,ls_misal_loads.ma64:u` tell instruction-bound from store-bound.
- Correctness: the differential fuzzer (qpdiff) against `qpref`: random bodies over the bytes that matter, every encode mode and both line endings against the old one-shot calls, decoding raw and encoded bodies, and random chunks with 1 to 300 bytes of room against the new one-shot; its release profile keeps debug assertions and overflow checks. Run 300,000 bodies before keeping a change, and `cargo test` in debug too. In the crate, `every_split_gives_the_same_output` and `a_frozen_corpus_keeps_its_output` (src/lib.rs) hold the same checks smaller.
- criterion (`cargo bench --features alloc --bench compare`, pinned) is what the READMEs publish; for this crate its 64-byte and 4 KiB cells swing by up to 2x between runs of the same binary (decoding ascii at 64 bytes read 1.79 in a full run, 3.2 GiB/s filtered alone) while encodify's hold within 5%, a placement effect the perf harness does not reproduce, even with criterion's buffer shapes; publish the median of three full runs and trust `perf` for anything closer than 2x.
- Assembly: `perf record -e cycles:u` and `perf annotate --stdio -s <function>` on the harness binary; samples pile on the instruction after a stall.

## Where the time goes

- Encoding binary: about 23 instructions an escape (tzcnt, blsr, the masked indices, a 32-byte run copy, the table word, `off += 2`, the line check). At 1 MiB, 51 misses a KiB: 32 are the block loop's exit, one a block, the rest the line check, once a line.
- Decoding binary: about 27 instructions an escape, but not bound by them: 37% of slots backend-bound on memory against encodify's 25%, three overlapping stores an escape (two 16-byte halves of the run copy, the byte) and about 1,200 store-to-load forwards a 4 KiB call from the outer loop's spills (encodify: 33). 16-byte run copies (wrong for long runs, measured only) were 7% faster. At 1 MiB the 78 misses a raw KiB are nearly all block-loop exits (32 a KiB of encoded input, 2.3x the raw size).
- Text rows: few events a block; the block scaffolding (about 45 instructions) and the 29-instruction mask dominate.

## Levers left

Ordered by expected gain, each to be measured before it is kept:

1. 64-lane blocks, measured whole this pass (v/s64): every 1 MiB row 5% to 25% faster and text rows at 4 KiB 8% to 12%, but every 64-byte row 10% to 65% slower and binary encoding slower. The 64-byte loss is the first block reading the 256-byte pad right after its stores (inputs under 128 bytes always read the pad) and the larger pad; the binary loss is the 64-byte run copies. To keep the gains: force an event at lane 32 so runs stay within 32-byte copies, and either build the pad with stores the window's loads can forward from or run 32-lane blocks below 128 bytes of input (more code: weigh the lines).
2. Decoding binary at 64 bytes: the store side (above): fewer live values in `blocks`' outer loop so nothing spills, fewer or narrower run stores when a cheap rule allows.
3. aarch64: never built (fenix's aarch64 rust-std and qemu in a throwaway shell, as base64-codec does). There is no `pmovmskb`: the masks may compile into a slow emulation, in which case a NEON `mask` (the `shrn` nibble mask, 4 bits a lane) is the lever, and with it `unsafe`.
4. SIMD past the baseline (SSSE3 `pshufb` compress and expand, AVX2, as yEnc decoders do): needs `unsafe` and a CPU token from the application, as base64-codec has; ask the user first, since every row already leads.
5. Fuzz targets, Miri, and the Q and DKIM modes (ask the user first: they may belong to another crate).

## Traps

- The mode must not be tested inside the walk: as a runtime flag it cost binary encoding 15% to 40% and a miss a line at 4 KiB. Dispatch with a `match` to `walk::<TEXT>`; a function-pointer array cost 15% more instructions, and so did the `breaks` closure reading `TEXT` instead of a `text` local.
- An edit outside the hot loop can wreck a row in one build: `finish` ending in `.inspect(|_| ..)` made encoding ascii 1.84 cycles a byte against 1.11 in the aligned build, same instructions; the same logic with `?` was fine.
- 64-lane blocks with 32-byte run copies corrupt runs over 32 bytes; a sentinel bit at lane 64 does not fit a u64 (63-lane encoding blocks did).
- `mask` must stay out of line and its halves inlined: inlined into a block, the masks merge into slow shuffles; over slices or with `(0..16).fold`, the halves go out of line with bounds checks. Folds over 32 lanes do not vectorize at all.
- Store forwarding: a buffer written with stores of one size and read with loads of another stalls. Build the pad before the blocks run, with whole 64-byte copies.
- The escape's validity must stay a branch: branch-free, the read cursor waits on two chained table loads and the walk serializes.
- `count_ones` is no single instruction on baseline x86-64 (no POPCNT); `trailing_zeros` is `rep bsf`, fine.
- A decoding block needs 64 bytes of input and output; an encoding block 64 bytes of input and the 160-byte `ROOM` of output, which is also the edge's queue and the slack in `encoded_len_max`: change them together, or the coroutine stops progressing or the one-shot encode runs out of room.
- Soft-break tails: a CR or LF right after `=` belongs to the soft break, and a bare CR right after a `=\n` soft break does not; a dropped bare CR does not end a run of blanks.
- SSE2 intrinsics need `unsafe` with rustc 1.99 although SSE2 is x86-64's baseline.
- In scripts, `perl s|..|..|` with `\|` inside is alternation, not a literal bar: edit code with the Edit tool. all.sh does not rebuild; run build.sh first.
