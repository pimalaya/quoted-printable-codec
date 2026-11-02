---
cairn: log
change: the-fused-spreads
landed: 2026-11-02
---

# The fused spreads

A base64-codec session (its log 2026-11-02-the-store-queue) found that this crate's `asm!` timing of `vpermt2b`, 0.8 a cycle on Zen 5, measured latency: the loop of the-wider-reach overwrote each `vpermt2b`'s first table, four chains of latency 5. Re-measured with ten independent destinations, `vpermt2b` and `vpermi2b` run 2 a cycle, as `vpermb` does; `vpcompressb` and `vpexpandb` run 1 a cycle, not 0.8, and wait on their destination register even zero-masked. The user asked to check this crate against those findings and adjust.

What landed: the AVX-512 encoder loads its tables plainly, where `core::hint::black_box` kept LLVM from fusing the three spread permutes of a group into two `vpermt2b`. Fused, a group takes one permute fewer and its chain to the compress one step shorter. The note claiming the fused form slower went with it.

Left as is: the kernels' `vpcompressb` compress in place, destination and source one register (the release assembly of both kernels), so the false dependency costs nothing here.

Cycles and instructions per byte, `perf stat` on core 13, averaged over the eight source and output offset pairs of the-column-chain, the least of two runs each, plain build then aligned build, the tree before (51dc98a) in brackets:

| workload | cycles | instructions |
| --- | --- | --- |
| encode ascii, 64 B | 1.260 / 1.262 (1.272 / 1.287) | 8.30 (8.28) |
| encode ascii, 4 KiB | 0.768 / 0.753 (0.778 / 0.760) | 3.91 (4.03) |
| encode ascii, 1 MiB | 0.716 / 0.719 (0.770 / 0.778) | 3.83 (3.94) |
| encode accented, 64 B | 2.021 / 1.981 (2.044 / 2.036) | 9.77 (9.94) |
| encode accented, 4 KiB | 0.731 / 0.729 (0.776 / 0.775) | 3.91 (4.03) |
| encode accented, 1 MiB | 0.716 / 0.723 (0.774 / 0.771) | 3.83 (3.94) |
| encode binary, 64 B | 1.427 / 1.476 (1.527 / 1.547) | 7.52 (7.70) |
| encode binary, 4 KiB | 0.438 / 0.442 (0.466 / 0.462) | 2.47 (2.57) |
| encode binary, 1 MiB | 0.447 / 0.438 (0.447 / 0.461) | 2.40 (2.49) |

Every row level or faster, 4% to 7% from 4 KiB on accented and binary and at 1 MiB on text. Decoding, the AVX2 and NEON backends and the portable one are unchanged. The library is one line shorter.

Checked: the suite in release, in debug with all features and in release with `-C target-cpu=native`, every backend the CPU has and the frozen corpus; clippy with `-D warnings` for all targets; the encode fuzz target for 3 minutes with inputs up to 8 KiB, 351,464 runs, clean; each encoding row checked against the portable backend by the measuring harness. Miri was not run: it has no AVX-512, and no other backend changed.

Capabilities moved: none, speed only.
