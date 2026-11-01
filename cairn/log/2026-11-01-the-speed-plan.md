---
cairn: log
change: the-speed-plan
landed: 2026-11-01
---

# The speed plan

Took over from the handoff of the-speed-plan, the scalar baseline at 0.07 to 0.23 GiB/s, last in every row. The user asked for more speed and less code, with free hands. Both directions now run 2x to 20x faster (criterion, benches/README.md), ahead of encodify 1.0.2 in every 1 MiB row and in ascii at 4 KiB, behind it in the six 64-byte rows, and in accented and binary at 4 KiB ahead by criterion, behind by 5% to 21% in cycle counts; the crate has no unsafe code and no CPU token, and its code grew from 422 to 507 lines (tests apart).

What landed:

- Blocks of 32 bytes in both directions. `mask` (src/lib.rs) builds a bitmask of the block's bytes in a class, 16 lanes a fold, which LLVM turns into `pcmpeqb` and `pmovmskb` on SSE2; a block walks the set bits of its special bytes only, copying each literal run between them with one 32-byte copy. Two bytes read ahead decide every escape, soft break, CRLF and line-final blank locally.
- The decoder's blocks take literal bytes, escapes, soft breaks, and CRLF or LF with no blank before them, with two masks (`=` and the plain bytes) and a scalar check per other special byte; anything else (a malformed escape, `=` and blanks, a bare CR, blanks before a line break, the input's end) gives the offset of its first byte, and the token steps, the scalar rules byte by byte, run past it. Escapes decode through a 256-byte `HEX` table.
- The encoder is blocks only, partial at the input's end: a blank before a line break becomes an event when the break's event finds it, or when the 2 bytes read ahead show it; its token path is gone.
- src/edge.rs replaces src/queue.rs and both per-byte state machines: the coroutines run blocks straight between the caller's buffers, and the bytes a block cannot decide yet (up to 78 decoding, 2 encoding) or has no room for go through a carry and a queue the core runs over. The one-shot decode calls the core directly; the one-shot encode runs the coroutine, as a block needs 134 bytes of room and `encoded_len_max` leaves less at the end.
- Semantics: a line break deletes the last 76 blanks before it and keeps the older ones, where the old decoder flushed all 76 at the 77th and carried on (78 blanks and CRLF decode to 2 blanks and CRLF, were 76). Every call may overwrite the bytes of `dst` past the ones it reports written.
- Tests: every split of 2000 random bodies, random chunks and room, both directions, against the one-shot calls; a frozen corpus hash, the old implementation's, which the new one matches; both fail on a reintroduced soft-break bug the fuzzer had found.

Tried and left out:

- SIMD runs per token (a 16-byte stop mask, runs copied until the next special byte): fast while the branch predictor memorizes the input (3.8 GiB/s ascii decoding at 64 B), one misprediction per run end at 1 MiB, and 65 instructions a byte encoding binary.
- Per-lane branch-free blocks (bool arrays, a compress or expand loop): 60 to 140 instructions a byte, LLVM's SLP vectorizer turning the unrolled lanes into shuffles. A dense compress, 5 instructions a lane, loses to the walk over set bits at every density measured.
- Masks folded over 32 lanes: not vectorized (155 instructions for the hex mask, 23 over 16 lanes). `mask` inlined: the block's masks merge into shuffles, 2x to 3x slower; it stays `#[inline(never)]`.
- Ten masks a block (CR, LF, blanks, the lookahead ones) to validate it: two masks and scalar checks per special byte took ascii decoding from 14 to 9 instructions a byte.
- A stack window for every block: store-forwarding stalls, 94% of the `=` mask's samples; blocks read the caller's input when 64 bytes remain.
- The decoder's block folded into `tokens`, as the one-caller rule asks: 70% slower on escapes (12.3 against 7.2 cycles a byte at 4 KiB), the loop out of registers; kept apart with a NOTE. The encoder's block and `escape` folded in for free.
- SSE2 intrinsics, though SSE2 is x86-64's baseline: rustc 1.99 asks `unsafe` for them unless the calling function enables the feature itself; not taken.

Found: Zen 5 memorizes the branch history of an input repeated up to about 64 KiB: no mispredictions at 64 bytes and 4 KiB for any library, 40 to 480 per KiB at 1 MiB, 9 at 256 KiB. The 64-byte and 4 KiB rows count instructions, the 1 MiB rows mispredictions; the first bench's slowdown of every library at 1 MiB was this, not the cache.

Capabilities moved: calls (MODIFIED: Lengths and short buffers, No dependency, no std), decoding (MODIFIED: Line breaks and trailing blanks, a scenario added).
