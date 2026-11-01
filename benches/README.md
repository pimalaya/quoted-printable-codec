# Benchmarks

[criterion](https://crates.io/crates/criterion) medians of `cargo bench --features std --bench compare` (source in [compare.rs](./compare.rs)), in GiB/s of raw bytes on both sides. Measured on 2026-11-01, rustc 1.99, Linux x86_64 on an AMD Ryzen AI Max+ 395 (Zen 5), pinned to one core with `taskset -c 13`, all groups in one run, on a machine shared with other work. Each cell is the median of three runs; "before" is the published median of the pass before (commit 4f932a9).

- quoted-printable-codec 0.0.1: the AVX-512 backend `QuotedPrintableCpu::detect` picks on this CPU, blocks of 64 bytes with the VBMI2 byte compress and no branch on the data; short text with few bytes to escape goes through the portable walk.
- encodify 1.0.2: `qp::BODY` (and `qp::BINARY` to encode binary), into a slice; it runs AVX2 here.
- quoted_printable 0.5.2: `encode`, `encode_binary` and `decode` in `ParseMode::Robust`; they allocate their output, as their API asks.
- mail-parser 0.11.9: `decoders::quoted_printable::quoted_printable_decode`, decoding only; it allocates its output.

Inputs: ascii is lowercase words with a few `.`, `,` and `=`, in CRLF lines of about 72 characters; accented is the same with about one character in seven a two-byte UTF-8 letter (é, è, à, ç, ê); binary is pseudo-random bytes, encoded in binary mode. Decoding reads quoted-printable-codec's encoding of each, and the bench checks that every decoder gives back the raw bytes.

## Encoding

| input | before | quoted-printable-codec | encodify | quoted_printable |
| --- | --- | --- | --- | --- |
| ascii, 64 B | 3.768 | 3.620 | 2.251 | 0.617 |
| ascii, 4 KiB | 3.993 | 4.260 | 2.221 | 0.727 |
| ascii, 1 MiB | 1.919 | 3.530 | 0.994 | 0.570 |
| ascii, 10 MiB | 1.723 | 4.373 | 1.042 | 0.576 |
| accented, 64 B | 1.394 | 2.051 | 0.967 | 0.395 |
| accented, 4 KiB | 1.606 | 4.374 | 1.082 | 0.481 |
| accented, 1 MiB | 0.923 | 4.444 | 0.427 | 0.304 |
| accented, 10 MiB | 0.945 | 4.398 | 0.421 | 0.306 |
| binary, 64 B | 1.208 | 2.681 | 0.643 | 0.237 |
| binary, 4 KiB | 1.328 | 5.960 | 0.682 | 0.259 |
| binary, 1 MiB | 0.857 | 5.526 | 0.201 | 0.156 |
| binary, 10 MiB | 0.913 | 4.225 | 0.201 | 0.146 |

## Decoding

| input | before | quoted-printable-codec | encodify | quoted_printable | mail-parser |
| --- | --- | --- | --- | --- | --- |
| ascii, 64 B | 2.492 | 3.415 | 2.746 | 0.270 | 0.955 |
| ascii, 4 KiB | 4.897 | 9.849 | 2.589 | 0.581 | 1.220 |
| ascii, 1 MiB | 1.881 | 10.114 | 1.181 | 0.472 | 0.523 |
| ascii, 10 MiB | 1.839 | 9.806 | 1.158 | 0.476 | 0.509 |
| accented, 64 B | 1.294 | 3.430 | 1.046 | 0.206 | 0.921 |
| accented, 4 KiB | 1.650 | 6.811 | 1.117 | 0.403 | 1.062 |
| accented, 1 MiB | 0.916 | 6.960 | 0.463 | 0.276 | 0.375 |
| accented, 10 MiB | 0.910 | 7.134 | 0.458 | 0.274 | 0.372 |
| binary, 64 B | 0.799 | 2.539 | 0.749 | 0.159 | 0.717 |
| binary, 4 KiB | 0.888 | 4.650 | 0.727 | 0.256 | 0.822 |
| binary, 1 MiB | 0.650 | 4.762 | 0.235 | 0.156 | 0.260 |
| binary, 10 MiB | 0.640 | 4.669 | 0.239 | 0.148 | 0.247 |

## Cycle counts

Cycles per byte of raw input from `perf stat`, averaged over eight placements of the source and output buffers, for both backends of this tree, the pass before (commit 4f932a9) and encodify; cairn/changes/the-wider-reach/proposal.md has them for two code layouts, with instructions and mispredictions. The portable backend is what runs on a CPU without AVX-512 VBMI2.

| workload | AVX-512 | portable | before | encodify |
| --- | --- | --- | --- | --- |
| encode ascii, 64 B | 1.22 | 1.18 | 1.17 | 1.96 |
| encode ascii, 4 KiB | 1.02 | 1.11 | 1.14 | 1.97 |
| encode ascii, 1 MiB | 1.02 | 2.18 | 2.18 | 4.37 |
| encode accented, 64 B | 2.23 | 3.00 | 3.08 | 4.82 |
| encode accented, 4 KiB | 0.98 | 2.59 | 2.57 | 4.25 |
| encode accented, 1 MiB | 1.00 | 4.17 | 4.22 | 10.92 |
| encode binary, 64 B | 1.68 | 3.07 | 3.53 | 7.37 |
| encode binary, 4 KiB | 0.74 | 3.18 | 3.20 | 6.69 |
| encode binary, 1 MiB | 0.77 | 4.34 | 4.56 | 20.69 |
| decode ascii, 64 B | 1.24 | 1.14 | 1.20 | 1.72 |
| decode ascii, 4 KiB | 0.45 | 0.85 | 0.90 | 1.72 |
| decode ascii, 1 MiB | 0.42 | 2.00 | 2.32 | 3.71 |
| decode accented, 64 B | 1.25 | 3.28 | 3.47 | 4.34 |
| decode accented, 4 KiB | 0.63 | 2.56 | 2.65 | 3.92 |
| decode accented, 1 MiB | 0.62 | 4.41 | 4.78 | 9.53 |
| decode binary, 64 B | 1.56 | 5.61 | 5.84 | 6.02 |
| decode binary, 4 KiB | 0.94 | 4.67 | 4.96 | 6.05 |
| decode binary, 1 MiB | 0.92 | 6.54 | 6.79 | 18.61 |

## Where it stands

Ahead of every other library in every row. The AVX-512 backend decodes 2x to 8x faster than the pass before from 4 KiB up, and 1.4x to 3.2x at 64 bytes; it encodes 1.8x to 6.4x faster from 1 MiB up and up to 4.5x at 4 KiB. Encoding ASCII at 64 bytes is level with the pass before (4% lower here, level by cycle counts), as it runs the same portable walk. Its kernels never branch on the data, so the 1 MiB and 10 MiB rows keep the speed of the 4 KiB ones, where the walk over special bytes mispredicts at the end of every block; it also makes their cells steady between runs, within 10% at 64 bytes and 4 KiB where the pass before moved by up to 2x.

The portable backend decodes up to 1.16x faster than the pass before (ASCII at 1 MiB), level with it on binary at 64 bytes, and encodes as fast.
