---
cairn: change
id: the-wider-reach
status: draft
created: 2026-11-01
---

# The wider reach

The handoff of the fifth session of 2026-11-01 (log 2026-11-01-the-wide-blocks), written for a fresh context to take over the optimisation without its history: where things stand, how to measure without fooling yourself, where the time goes, what was tried, and what is left.

## Why

The user's goal, as for base64-codec (../base64-codec): beat encodify 1.0.2 in every section of benches/compare.rs with the most concise code, each pass better than the last in speed and in size, free hands, searching online for new methods where needed. This pass answered the user's question "why is SIMD not used?" with an AVX-512 backend: every 4 KiB and 1 MiB row runs 1.07x to 8x faster than the pass before by cycle counts, the 64-byte rows of accented and binary bodies 1.4x to 3.8x, and the two 64-byte ASCII rows are level with it (within 5%, either side by build); every row leads encodify, by 1.4x to 27x. The cost is size: 820 lines against 506 (non-blank, non-comment lines before each `#[cfg(test)]` in src/*.rs, after `cargo fmt`), src/avx512.rs and src/cpu.rs being 281 of them. No published SIMD quoted-printable codec was found to learn from; node-yencode and rapidyenc (yEnc, with an AVX-512 VBMI2 target) and Lemire's simdprune (byte compress without VBMI2) are the closest relatives.

Settled so far: the CPU token is the application's, as in base64-codec (user, 2026-10-31): `QuotedPrintableCpu`, built by the `const fn` `QuotedPrintableCpu::new` from the application's shared `simd_cpu::SimdCpu` (a-shared-cpu-token, 2026-11-01), held by the codec; unsafe code serves only the AVX-512 backend, each block with a one-line `SAFETY` comment (`clippy::undocumented_unsafe_blocks` is denied); the portable backend stays as fast as before, as most x86 laptops (Intel since Alder Lake) and every aarch64 machine run it; the decoder's blank cap slides; every call may overwrite `dst` past what it reports, within its length; `encoded_len_max` includes the 200 bytes an encoding chunk may touch past its output; a successful `finish` resets a coroutine; lowercase hex digits decode but report unclean; a malformed escape keeps `=` and its hex digit and decodes what follows again; decoding never fails; names use `QuotedPrintable`, never `Qp`.

## Where things stand

The code: src/cpu.rs holds the token and `QuotedPrintableCpu::wide`, which tells the encoder whether to run the AVX-512 groups (its backend, and an input over 64 bytes or with more than 4 bytes to escape: on fewer, the portable walk over them is faster). src/avx512.rs holds both kernels. `decode` reads blocks of 64 bytes three times, at 0, 1 and 2 bytes on (masked loads at the input's end), finds escapes as `=` with two hex digits (a `vpermb` lookup on the low 6 bits checked against the byte), decodes all of them with two more `vpermb` (high and low nibble values), blends them in and compresses away their digits and soft breaks (`=` LF, `=` CRLF) with `vpcompressb`; an escape or soft break running past the block carries its deleted lanes into the next block; it stops at the first byte the token steps decide (a malformed `=`, a bare CR, a line break after a blank), and advances 64 bytes a block through a predicted branch. `encode` takes chunks of 63 bytes with masks computed once (bytes to escape, line feeds, CRs dropped before them, blanks before a line break or the input's end), and three groups of 21 bytes each: `vpermb` spreads each byte over three lanes, a nibble blend and a `vpermb` give the hex digits, masked moves put `=` and the CR of a CRLF line break, the keep mask is `STARTS | pdep(esc) << 1 | pdep(esc) << 2` (with the CR dropped or added), `vpcompressb` packs the output; `pext` moves the literal, escape and line-feed starts to output positions, which give the one soft break a group may need and the next column (`lzcnt` of the line starts), with no branch on the data; three stores write a group (the vector, the soft break, the vector again past it under a mask). The input's end is one masked partial chunk, with the coroutine's hold logic. src/decode.rs `tokens` runs either backend's blocks, then the token steps; it counts the trailing blanks after each blocks call for both backends, and fills the pad only for the portable blocks. The portable code is the pass before's, the decoder's blocks no longer counting blanks.

Cycles per byte of raw input (`perf stat`, core 13, averaged over eight source and output offset pairs, plain build then aligned build), the AVX-512 backend (`detect` on this Zen 5) and the portable one against the pass before (commit 4f932a9) and encodify:

| workload | AVX-512 cycles | instructions | misses per KiB | portable cycles | pass 3 cycles | encodify cycles | encodify misses per KiB |
| --- | --- | --- | --- | --- | --- | --- | --- |
| encode ascii, 64 B | 1.22 / 1.21 | 7.5 | 0 | 1.18 / 1.19 | 1.17 / 1.20 | 1.96 / 1.95 | 0 |
| encode ascii, 4 KiB | 1.02 / 1.03 | 4.3 | 0 | 1.11 / 1.10 | 1.14 / 1.11 | 1.97 / 1.96 | 0 |
| encode ascii, 1 MiB | 1.02 / 1.00 | 4.2 | 0 | 2.18 / 2.36 | 2.18 / 2.32 | 4.37 / 4.33 | 80 |
| encode accented, 64 B | 2.23 / 2.23 | 10.1 | 0 | 3.00 / 2.98 | 3.08 / 3.07 | 4.82 / 4.74 | 2 |
| encode accented, 4 KiB | 0.98 / 1.00 | 4.3 | 0 | 2.59 / 2.60 | 2.57 / 2.60 | 4.25 / 4.24 | 0 |
| encode accented, 1 MiB | 1.00 / 1.00 | 4.2 | 0 | 4.17 / 4.43 | 4.22 / 4.45 | 10.92 / 10.91 | 224 |
| encode binary, 64 B | 1.68 / 1.66 | 7.3 | 0 | 3.07 / 3.47 | 3.53 / 3.52 | 7.37 / 7.32 | 0 |
| encode binary, 4 KiB | 0.74 / 0.74 | 2.6 | 0 | 3.18 / 3.15 | 3.20 / 3.23 | 6.69 / 6.87 | 0 |
| encode binary, 1 MiB | 0.77 / 0.81 | 2.5 | 0 | 4.34 / 4.47 | 4.56 / 4.61 | 20.69 / 21.52 | 479 |
| decode ascii, 64 B | 1.24 / 1.26 | 6.3 | 0 | 1.14 / 1.19 | 1.20 / 1.24 | 1.72 / 1.82 | 0 |
| decode ascii, 4 KiB | 0.45 / 0.43 | 1.7 | 0 | 0.85 / 0.84 | 0.90 / 0.92 | 1.72 / 1.73 | 0 |
| decode ascii, 1 MiB | 0.42 / 0.41 | 1.6 | 0 | 2.00 / 2.02 | 2.32 / 2.38 | 3.71 / 3.74 | 74 |
| decode accented, 64 B | 1.25 / 1.26 | 6.3 | 0 | 3.28 / 3.37 | 3.47 / 3.57 | 4.34 / 4.41 | 0 |
| decode accented, 4 KiB | 0.63 / 0.64 | 2.4 | 0 | 2.56 / 2.57 | 2.65 / 2.72 | 3.92 / 3.92 | 0 |
| decode accented, 1 MiB | 0.62 / 0.61 | 2.3 | 0 | 4.41 / 4.44 | 4.78 / 4.92 | 9.53 / 9.51 | 212 |
| decode binary, 64 B | 1.56 / 1.57 | 7.8 | 0 | 5.61 / 6.03 | 5.84 / 6.03 | 6.02 / 6.04 | 0 |
| decode binary, 4 KiB | 0.94 / 0.97 | 3.6 | 0 | 4.67 / 4.80 | 4.96 / 4.93 | 6.05 / 6.05 | 0 |
| decode binary, 1 MiB | 0.92 / 0.93 | 3.5 | 0 | 6.54 / 6.58 | 6.79 / 7.03 | 18.61 / 18.45 | 463 |

Criterion medians are in README.md and benches/README.md.

## How to measure

- Cycles, instructions and branch misses per byte, not time: `perf stat -x, -e cycles:u,instructions:u,branch-misses:u` (perf from `nix-shell -p perf`) over a binary running one call N times, at N and 2N, the difference over N times the size, pinned with `taskset -c 13` (its sibling is 29; check both idle with `ps -eo psr,pcpu,comm`). Start fuzzers and builds pinned away (`taskset -c 2-11`), and never build or run perf while criterion runs.
- The harness, in a scratch crate depending on the tree and on simd-cpu by path, simd-cpu with its `std` feature, on a copy of the tree before the session (`git archive HEAD src Cargo.toml`, package renamed `qpref`, `[[bench]]` and dev-dependencies dropped) and on encodify `=1.0.2`: `qpbench <qp|qpp|ref|enc> <encode|decode> <ascii|accented|binary> <size> <iters> [src offset] [dst offset]`, `qp` the detected backend, `qpp` the portable one through `QuotedPrintableCpu::new(SimdCpu::NONE)` (the harness of 2026-11-01 still calls the removed `assume` and `detect`), inputs generated as benches/compare.rs does, buffers at those offsets from a page boundary. The differential fuzzer `qpdiff <bodies> <a|p>` checks one backend against `qpref`: random bodies over the bytes that matter, every encode mode and both line endings against the one-shot calls, decoding raw and encoded bodies, random chunks with 1 to 300 bytes of room; its release profile keeps debug assertions and overflow checks. This session's copies are in /tmp/claude-1000/-home-soywod-code-pimalaya/4a97e696-fd7c-47a6-9902-7393d997a841/scratchpad while /tmp lasts: build.sh (both bench builds and the fuzzer; `NOFUZZ=1` skips the fuzzer), pstat.sh (one row, `T=target-aligned` for the aligned build), table.sh (rows by `LIBS`, `OPS`, `KINDS`, `SIZES`, `PAIRS`, `T`), loc.sh (the line count), crit.sh (three criterion runs), base-*.txt and fin-*.txt (the tables before and after this pass).
- Build the harness twice, plain and with `RUSTFLAGS='-C llvm-args=-align-all-functions=6 -C llvm-args=-align-loops=64'` into another target directory, and read both after every edit: layout moves the 64-byte rows by 10% and more.
- The 64-byte and 4 KiB rows count instructions (Zen 5 memorizes the branch history of an input repeated up to about 64 KiB), the 1 MiB rows count mispredictions; the AVX-512 kernels have none, so their 1 MiB rows follow their 4 KiB ones.
- The offset pairs matter: a 64-byte AVX-512 encode with source and output at the same page offset (pair 0,0) ran 2x slower than the others (4K aliasing of its loads against the last group's stores). Average the eight pairs, and look at pair 0,0 alone when a small row moves.
- Correctness: `cargo test` in release and debug runs every test under every backend the CPU has (`cpu::available`), and the frozen corpus hash under each; run `qpdiff 300000 a` and `qpdiff 300000 p` before keeping a change, and the tests once with `RUSTFLAGS='-C target-cpu=native'` (the AVX-512 backend as `BASELINE`).
- criterion (`cargo bench --bench compare`, pinned) is what the READMEs publish; its 64-byte and 4 KiB cells swing by up to 2x for this crate between runs: publish the median of three full runs and trust `perf` for anything closer.
- Assembly: `objdump -d --no-show-raw-insn -M intel` (binutils from `nix-shell -p binutils`) on the harness binary; per-size instruction counts (sizes 1, 21, 64, 128...) show the fixed and per-group costs better than `perf annotate`, whose instruction samples are few at 64 bytes.

## Where the time goes

- AVX-512 encoding: about 88 instructions a group of 21 bytes in text (60 in binary), whatever the content: 3 `pdep`, 3 `pext`, 8 vector operations, the cut and column arithmetic (about 20), the stores. The 63-byte chunk shares the masks (about 30). The column is a dependency chain of about 10 cycles a group; the loads never wait on data.
- AVX-512 decoding: about 85 instructions a block of 64 encoded bytes; the binary rows decode 2.3 encoded bytes per raw byte, hence 3.5 instructions a raw byte.
- The 64-byte rows: a call costs about 150 instructions before any byte (harness, prologue, constants). A 64-byte input is a full chunk and a one-byte partial chunk when encoding (hence the portable walk on sparse text), two blocks when decoding (66 encoded bytes).
- The portable backend is the pass before's: its 1 MiB rows count mispredictions at block-loop exits (33 to 80 a KiB).

## Levers left

Ordered by expected gain, each to be measured before it is kept:

0. Done on 2026-11-01, the change a-shared-cpu-token: the codec takes `QuotedPrintableCpu::new(simd_cpu::SimdCpu)`, the AVX-512 backend when the token holds `Avx512Vbmi2`, `Avx512Vbmi`, `Bmi1`, `Bmi2`, `Lzcnt` and `Popcnt`; `BASELINE`, `assume`, `detect` and the `std` feature are gone, and the library's release assembly kept every function as it was.
1. An AVX2 backend, for the x86 CPUs without AVX-512 (Intel client CPUs since Alder Lake, Zen 2 and 3): decoding with 32-byte blocks, `vpshufb` lookups and a compress through a table of shuffles indexed by 8-bit masks (simdprune); encoding with the three-lane spread over 128-bit lanes and the same compress, or a 2-byte `vpshufb` expansion. Weigh its lines against the portable rows it replaces.
2. A NEON backend for aarch64 (Apple, Graviton): `tbl` lookups, no compress instruction; and an aarch64 build of the portable one, never made (fenix's aarch64 rust-std and qemu in a throwaway shell, as base64-codec does).
3. The 64-byte ASCII rows, level with the pass before: encoding pays `wide`'s 64-byte check (about 30 instructions) on top of the portable walk; decoding pays two AVX-512 blocks for 66 bytes, where the portable blocks are 8% faster on that input. A cheaper check, or one chunk covering 64 bytes, would decide them.
4. A sparse encoder for text: with few escapes, `vpexpandb` on the output (input bytes at their output positions, `=` and digits placed by mask) takes about 60 input bytes per 64 output bytes where the three-lane spread takes 21; the output positions come from the same `pdep`/`pext` masks. The input's advance then depends on the data: keep the loads off that chain.
5. Size: 820 lines. Candidates: the encoder's partial chunk (25 lines) folded into the chunk loop costs 20% to 40% (measured); the decoder's two load paths, folded into masked loads, 10% to 20% (measured); find cuts that keep the speed.
6. Miri on the portable backend (it has no unsafe code), fuzz targets, and the Q and DKIM modes (ask the user first: they may belong to another crate).

## Traps

- A data-dependent cursor serializes blocks: `read += stop.min(rest)` or `read += commit.count_ones()` made the decoder 1.6x slower; advance by 64 and leave on a predicted branch.
- A closure with two call sites can go out of line in one mode only: the encoder's chunk closure did in text mode (70% more instructions); keep one call site, or check the symbols with `nm -C` for `{closure#`.
- Per-group lengths computed at run time cost 20% to 40% of encoding: the full chunks use constant 21-byte groups and one room check (`ROOM`, 200 bytes, what three groups touch); only the partial chunk computes lengths.
- Masked loads from past the slice end must use `wrapping_add`, not `add`, for the address; `1 << (rest - 1)` must be guarded against `rest` over 64 (the fuzzer's overflow checks caught it).
- A density check costs about 30 instructions a call with one 64-byte load, 50 with two: a loss when the check is wrong as often as right (it was in the decoder).
- The encoder's hold logic must look at the true end of `src`, then cap the length to 63: capping first held the wrong bytes.
- `vpcompressb` to memory is slow on Zen 4; compress to a register, then store.
- Every trap of the pass before still holds for the portable backend (log 2026-11-01-the-lean-pass and the archived the-wide-blocks): the mode as a const generic, `mask` out of line with its halves inlined, store forwarding from the pad, layout effects of cold-path edits.
