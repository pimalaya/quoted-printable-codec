---
cairn: log
change: the-small-calls
landed: 2026-11-01
---

# The small calls

Took over from the handoff of the-small-calls: ahead of encodify 1.0.2 at 1 MiB and in ascii at 4 KiB, behind it in the six 64-byte rows and, by cycle counts, in accented and binary at 4 KiB. The user asked for more speed and less code, free hands. Both directions now lead encodify in all 18 rows by cycle counts (`perf`, eight source and output offset pairs), by 8% (decoding binary at 64 bytes) to 3.7x (encoding binary at 1 MiB), and take 1.1x to 2.4x fewer cycles a byte than the pass before; the code went from 507 to 526 lines (tests apart, `cargo fmt`), still no unsafe code and no CPU token.

What landed:

- `mask` (src/lib.rs) folds two fixed 16-byte halves inlined into it. Over slices, each mask was three calls, the two halves out of line with bounds checks around 3 useful instructions; it is now 29 straight-line instructions. `Range::fold` stays out of line even in an `#[inline(always)]` function; the slice iterator's fold inlines.
- Decoding: `blocks` (src/decode.rs) is one call running blocks of 32 bytes straight over the caller's buffers, with no window or spare zeroed per block. One walk over the special bytes in order decodes escapes and soft breaks, keeps CRLF and LF with no blank before them, and ends the block at the first byte it leaves to the token steps (a malformed escape, a bare CR, a line break after a blank), committing what comes before it; the separate pass over the other special bytes is gone. An escape branches on its digits being valid, so the read cursor advances by 3 without waiting on the two chained table loads (binary at 4 KiB from 7.4 to 5.0 cycles a byte, the largest single gain). The output cursor is the read cursor minus the bytes deleted so far. `escape` reads a two-row `u16` digit table whose OR is the decoded byte, with flags for lowercase and for a non-digit. The last 8 bytes go to the token steps.
- The end of an input: `pad` copies its last 64 bytes, whole, ending at offset 64 of a zeroed 128-byte buffer, and `window` reads a block's 64 bytes from it when fewer remain. The pad is built before the blocks run, so its stores have retired when blocks load it: built just before, the blocks' loads stalled on store forwarding (a 64-byte decode spent 29% of its cycles in the `=` mask that way). The decoder's output goes through a 64-byte spare only when `dst` has fewer than 64 bytes left.
- Encoding: the same pad and window; the one-shot `encode` runs `blocks` directly with the input's end known, and only the last blocks, when `dst` has under 134 bytes left, through a spare; the coroutine is for chunked calls only. Escapes and line breaks take separate branches; escapes copy a 4-byte word from a 256-entry table (10% faster in binary than computing the digits).
- `decode` may overwrite the whole of `dst`, not only `decoded_len_max` bytes of it, as the spec already allowed: callers with slack get the tail decoded in place.
- Smaller: the decoder keeps a `dirty` flag and derives `Default`, `Edge` implements `Default`, the soft break is a padded 4-byte word with its length, `decode_vec` calls `decode`, and the encoder's `blocks` returns the edge's triple.

Tried and left out:

- A branch-free soft break before each escape: more instructions, slower in every row.
- The hex digits computed per escape instead of the table: binary encoding 10% slower.
- Token steps for every tail under 64 bytes: 5x slower at 64 bytes, as `tokens` called `blocks` after each step.
- A tail chunk built in a `u128` (two 8-byte stores): blocks' 16-byte loads then stall on store forwarding.
- Per-block room estimated from the event count, so that the last blocks of a one-shot encode fit `dst` without the spare: `count_ones` is no single instruction on baseline x86-64 and the slice drops the fixed-size array; 4 KiB rows 5% to 10% slower.
- The block's end computed from its last 3 bytes apart from the walk, so the next block's loads need not wait: no gain, more instructions.
- The two-pass block (escapes, then the other special bytes) kept beside the ordered walk: as fast at 4 KiB, 20 lines more.

Found: the code layout of a build moves a row by up to 20% (decoding accented at 64 bytes: 3.2 or 3.8 cycles a byte between the aligned and plain builds of the same source), and a process landing on core 13 once made a row 60% slower for one run; build the harness a second time with `RUSTFLAGS='-C llvm-args=-align-all-functions=6 -C llvm-args=-align-loops=64'` and read both before keeping a change. criterion's 64-byte rows swing by about 6% run to run. Size: the 19 lines over the pass before are the escape table (6 more than computing the digits), the two-row digit table and `escape` (4 more than `HEX`), the one-shot encode's spare (7) and the block loops' scaffolding, less the cuts above; the tables and the spare each measured as worth their lines.

Capabilities moved: none.
