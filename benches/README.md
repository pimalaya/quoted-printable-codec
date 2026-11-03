# Benchmarks

[criterion](https://crates.io/crates/criterion) medians of `cargo bench --bench compare` (source in [compare.rs](./compare.rs)), in GiB/s of raw bytes on both sides. Measured on 2026-11-01, rustc 1.99, Linux x86_64 on an AMD Ryzen AI Max+ 395 (Zen 5), pinned to one core with `taskset -c 13`, all groups in one run, on a machine shared with other work. Each cell is the median of three runs, quoted-printable-codec's encoding cells moving by up to 4% between runs and its decoding cells by up to 11%; "before" is the published median of the pass before (commit 72d8dc6, whose code 2c7604e kept).

- quoted-printable-codec 0.0.1: the AVX-512 backend `QuotedPrintableCpu::new(SimdCpu::detect())` picks on this CPU, blocks of 64 bytes with the VBMI2 byte compress, no branch on the data and each block read ahead; short text with few bytes to escape goes through the portable walk. A CPU without AVX-512 VBMI2 runs the AVX2 backend, and aarch64 the NEON one; neither is measured yet.
- encodify 1.0.2: `qp::BODY` (and `qp::BINARY` to encode binary), into a slice; it runs AVX2 here.
- quoted_printable 0.5.2: `encode`, `encode_binary` and `decode` in `ParseMode::Robust`; they allocate their output, as their API asks.
- mail-parser 0.11.9: `decoders::quoted_printable::quoted_printable_decode`, decoding only; it allocates its output.

Inputs: ascii is lowercase words with a few `.`, `,` and `=`, in CRLF lines of about 72 characters; accented is the same with about one character in seven a two-byte UTF-8 letter (é, è, à, ç, ê); binary is pseudo-random bytes, encoded in binary mode. Decoding reads quoted-printable-codec's encoding of each, and the bench checks that every decoder gives back the raw bytes.

## Encoding

| input | before | quoted-printable-codec | encodify | quoted_printable |
| --- | --- | --- | --- | --- |
| ascii, 64 B | 3.620 | 3.653 | 2.338 | 0.649 |
| ascii, 4 KiB | 4.260 | 5.642 | 2.295 | 0.750 |
| ascii, 1 MiB | 3.530 | 5.763 | 1.045 | 0.580 |
| ascii, 10 MiB | 4.373 | 5.760 | 1.033 | 0.585 |
| accented, 64 B | 2.051 | 2.156 | 0.987 | 0.393 |
| accented, 4 KiB | 4.374 | 5.720 | 1.087 | 0.494 |
| accented, 1 MiB | 4.444 | 5.771 | 0.427 | 0.308 |
| accented, 10 MiB | 4.398 | 5.747 | 0.423 | 0.306 |
| binary, 64 B | 2.681 | 2.873 | 0.648 | 0.246 |
| binary, 4 KiB | 5.960 | 9.460 | 0.688 | 0.258 |
| binary, 1 MiB | 5.526 | 9.584 | 0.217 | 0.158 |
| binary, 10 MiB | 4.225 | 9.139 | 0.217 | 0.145 |

## Decoding

| input | before | quoted-printable-codec | encodify | quoted_printable | mail-parser |
| --- | --- | --- | --- | --- | --- |
| ascii, 64 B | 3.415 | 3.359 | 2.794 | 0.332 | 1.260 |
| ascii, 4 KiB | 9.849 | 10.855 | 2.601 | 0.589 | 1.313 |
| ascii, 1 MiB | 10.114 | 11.867 | 1.199 | 0.487 | 0.534 |
| ascii, 10 MiB | 9.806 | 10.417 | 1.172 | 0.475 | 0.527 |
| accented, 64 B | 3.430 | 3.368 | 1.070 | 0.206 | 0.933 |
| accented, 4 KiB | 6.811 | 7.446 | 1.142 | 0.412 | 1.009 |
| accented, 1 MiB | 6.960 | 7.621 | 0.469 | 0.272 | 0.377 |
| accented, 10 MiB | 7.134 | 7.197 | 0.466 | 0.272 | 0.372 |
| binary, 64 B | 2.539 | 2.518 | 0.751 | 0.155 | 0.683 |
| binary, 4 KiB | 4.650 | 5.431 | 0.737 | 0.259 | 0.772 |
| binary, 1 MiB | 4.762 | 5.508 | 0.243 | 0.154 | 0.250 |
| binary, 10 MiB | 4.669 | 4.733 | 0.242 | 0.143 | 0.246 |

## Cycle counts

Cycles per byte of raw input from `perf stat`, averaged over eight placements of the source and output buffers, for both backends of this tree, the AVX-512 backend of the tree before (commit 2c7604e) and encodify; cairn/changes/the-column-chain/proposal.md has them for two code layouts, with instructions and mispredictions. The portable backend is what ran on a CPU without AVX-512 VBMI2 until the AVX2 and NEON backends came on 2026-11-02, built for accuracy first and not measured yet.

| workload | AVX-512 | portable | before | encodify |
| --- | --- | --- | --- | --- |
| encode ascii, 64 B | 1.19 | 1.15 | 1.27 | 1.99 |
| encode ascii, 4 KiB | 0.78 | 1.12 | 1.00 | 1.97 |
| encode ascii, 1 MiB | 0.77 | 2.18 | 1.00 | 4.20 |
| encode accented, 64 B | 2.08 | 3.03 | 2.18 | 4.69 |
| encode accented, 4 KiB | 0.79 | 2.59 | 0.98 | 4.25 |
| encode accented, 1 MiB | 0.77 | 4.18 | 1.00 | 10.59 |
| encode binary, 64 B | 1.51 | 3.46 | 1.73 | 7.37 |
| encode binary, 4 KiB | 0.46 | 3.23 | 0.74 | 6.66 |
| encode binary, 1 MiB | 0.46 | 4.36 | 0.77 | 21.00 |
| decode ascii, 64 B | 1.32 | 1.21 | 1.27 | 1.71 |
| decode ascii, 4 KiB | 0.43 | 0.84 | 0.45 | 1.73 |
| decode ascii, 1 MiB | 0.38 | 2.02 | 0.43 | 3.72 |
| decode accented, 64 B | 1.28 | 3.33 | 1.39 | 4.27 |
| decode accented, 4 KiB | 0.61 | 2.58 | 0.66 | 3.90 |
| decode accented, 1 MiB | 0.54 | 4.47 | 0.64 | 9.55 |
| decode binary, 64 B | 1.56 | 5.84 | 1.60 | 6.00 |
| decode binary, 4 KiB | 0.82 | 4.78 | 0.91 | 6.04 |
| decode binary, 1 MiB | 0.81 | 6.60 | 0.90 | 18.39 |

## Where it stands

Ahead of every other library in every row. The AVX-512 backend encodes 1.3x to 1.6x faster than the pass before from 4 KiB up on text and 1.6x to 2.2x on binary, as each chunk builds the bytes an escape takes once and reads the next chunk ahead of the soft-break logic of this one; it decodes 1.09x to 1.17x faster from 4 KiB to 1 MiB, each block scanned ahead of the one it cuts, and 1.01x to 1.06x at 10 MiB. The 64-byte cells are level with the pass before: the encoding ones 1% to 7% higher, the decoding ones 1% to 2% lower, within the 5% to 11% the decoding cells moved between runs; by cycle counts too, the 64-byte rows are level or ahead within the 5% to 7% they move between runs.

The portable backend is the pass before's.
