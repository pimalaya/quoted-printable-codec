---
cairn: log
change: the-lean-pass
landed: 2026-11-01
---

# The lean pass

Took over from the handoff of the-lean-pass: ahead of encodify 1.0.2 in all 18 rows by cycle counts, decoding binary at 64 bytes the tightest, 531 lines. The user asked again for more speed and less code, free hands, then for a README shaped as base64-codec's with the same benchmark sizes. Encoding takes up to 1.75x fewer cycles a byte than the pass before (binary at 4 KiB from 5.6 to 3.2, 2.1x ahead of encodify; ascii level); decoding is level with the pass before within noise (accented at 64 bytes 10% behind it in the aligned build only), and decoding binary at 64 bytes is level with encodify by cycle counts, its lead of the pass before gone; the code went from 531 to 506 lines (tests apart, `cargo fmt`), still no unsafe code and no CPU token.

What landed:

- Encoding: `blocks` dispatches on the mode to `walk::<TEXT>`, the loop compiled once per mode. With the mode a runtime flag, the compiler kept it on the stack and materialized the line-break test on every event: 15% to 40% more cycles in binary, and the 35 misses a KiB at 4 KiB the handoff could not locate. A function-pointer dispatch cost 15% more instructions; so did the `breaks` closure reading `TEXT` instead of a `text` local.
- Encoding: the walk tracks `off`, the output offset minus the input offset, biased by 128 so that dropped CRs never take it negative and `& 127` erases the bias, and `lim`, the output offset where the line is full, in place of a column: one check per event (`n + 3 > lim`, with the run end or escape it guards) leads to the rare path that breaks the line inside the run or before the escape. The block's end is a sentinel bit, its event an escape written past the output, so the loop needs no end test. Every index into the 160-byte output window and the 64-byte input window is masked, which removed the twelve bounds checks of an escape: 23 instructions an escape against 46.
- `encoded_len_max` adds the 160 bytes a block may touch past its output, so the one-shot `encode` runs `blocks` straight to the end: the spare buffer, its `memset` and `memmove` (a quarter of a 64-byte binary encode) and 7 lines are gone. The spec's calls capability says it.
- Decoding: blocks run to the input's end on the pad, no longer leaving the last 8 bytes to the token steps; the steps then only see a malformed `=`, a CR, an LF or the end, so their escape and plain-byte arms and the `irregular` guard went. The walk uses the biased offset too.
- A successful `finish` resets the coroutine (the encoder's column, the decoder's dirty flag; the edge already reset itself), with `a_finished_coroutine_starts_over`, which fails on the tree before.
- benches/compare.rs measures 10 MiB too; README.md, new, holds base64-codec's tables.

Tried and left out:

- 64-lane blocks (u64 masks from four 16-lane folds, a 128-byte window, a 256-byte pad, 64-byte run copies, 63-lane encoding blocks for the sentinel), whole, in both directions: every 1 MiB row 5% to 25% faster (one block-loop exit misprediction per 64 bytes instead of 32) and text rows at 4 KiB 8% to 12%, but every 64-byte row 10% to 65% slower (the first block reads the pad right after its stores) and binary encoding slower (64-byte copies). The handoff the-wide-blocks says how it could be kept.
- The column-based walk with masked indices and two line checks: fewer instructions in text, more cycles in binary than the offset walk.
- Decoding: `blanks` in a local (more instructions); lowercase escapes sent to the token steps to drop `flags |= value` (3% fewer instructions, no fewer cycles, 3 more lines). 16-byte run copies (wrong for long runs, measured only) were 7% faster in binary: stores are a share of the decoder's time, not the wall.

Found: decoding binary is bound by neither instructions nor branches but by the store side: 37% of slots backend-bound on memory against encodify's 25%, three overlapping stores an escape and about 1,200 store-to-load forwards a 4 KiB call from the outer loop's spills (encodify: 33); `perf stat -M PipelineL1,PipelineL2` and `ls_stlf` tell it. An edit outside the hot loop (`finish` ending in `.inspect(..)`) moved encoding ascii in the aligned build from 1.11 to 1.84 cycles a byte with the same instructions; the same logic written with `?` did not. criterion's 64-byte cells move with the allocator's placement far more than the handoff said: encoding binary at 64 bytes read 0.61 GiB/s in one full run and 1.23 alone.

Criterion, the tree before in one run, this tree the median of three (64 bytes to 1 MiB; 10 MiB in two runs and the median of three for this tree), in GiB/s: encoding binary at 64 bytes, 4 KiB, 1 MiB, 10 MiB 0.74, 0.82, 0.75, 0.70 before and 1.21, 1.33, 0.86, 0.91 now; accented 1.22, 1.35, 0.76, 0.81 and 1.39, 1.61, 0.92, 0.94; ascii 3.84, 2.85, 1.58, 1.83 and 3.77, 3.99, 1.92, 1.72. Decoding ascii 3.46, 4.92, 1.90, 1.87 and 2.49, 4.90, 1.88, 1.84; accented 1.39, 1.66, 0.91, 0.92 and 1.29, 1.65, 0.92, 0.91; binary 0.80, 0.81, 0.51, 0.65 and 0.80, 0.89, 0.65, 0.64. Decoding ascii at 64 bytes reads 28% slower in criterion and 10% faster by cycle counts over eight buffer placements; its three runs read 2.49, 1.79 and 3.24 GiB/s, so criterion's placement effect (above) decides that cell. The medians lead encodify in every cell, by 6% (decoding binary at 64 bytes) to 4.3x (encoding binary at 10 MiB); README.md has them.

Checked: the test suite in release and debug with all features, the library without default features, clippy for all targets with `-D warnings`, rustdoc with `-D warnings`, 300,000 differential fuzz bodies against the tree before, `a_frozen_corpus_keeps_its_output` unchanged.

Capabilities moved: calls (the room `encoded_len_max` gives, a coroutine reused after `finish`).
