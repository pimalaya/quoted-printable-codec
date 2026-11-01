# Benchmarks

[criterion](https://crates.io/crates/criterion) medians of `cargo bench --features alloc --bench compare` (source in [compare.rs](./compare.rs)), in GiB/s of raw bytes on both sides. Measured on 2026-11-01, rustc 1.99, Linux x86_64 on an AMD Ryzen AI Max+ 395 (Zen 5), pinned to one core with `taskset -c 13`, all groups in one run, on a machine shared with other work.

- quoted-printable-codec 0.0.1: blocks of 32 bytes walked by bitmask, safe code the compiler vectorizes (SSE2 here); "before" is the scalar byte-by-byte implementation of the first commit, from the first bench of the same day.
- encodify 1.0.2: `qp::BODY` (and `qp::BINARY` to encode binary), into a slice.
- quoted_printable 0.5.2: `encode`, `encode_binary` and `decode` in `ParseMode::Robust`; they allocate their output, as their API asks.
- mail-parser 0.11.9: `decoders::quoted_printable::quoted_printable_decode`, decoding only; it allocates its output.

Inputs: ascii is lowercase words with a few `.`, `,` and `=`, in CRLF lines of about 72 characters; accented is the same with about one character in seven a two-byte UTF-8 letter (é, è, à, ç, ê); binary is pseudo-random bytes, encoded in binary mode. Decoding reads quoted-printable-codec's encoding of each, and the bench checks that every decoder gives back the raw bytes.

## Encoding

| input | before | quoted-printable-codec | encodify | quoted_printable |
| --- | --- | --- | --- | --- |
| ascii, 64 B | 0.215 | 1.702 | 2.338 | 0.628 |
| ascii, 4 KiB | 0.191 | 3.250 | 2.296 | 0.755 |
| ascii, 1 MiB | 0.178 | 1.685 | 1.073 | 0.577 |
| accented, 64 B | 0.231 | 0.829 | 0.990 | 0.397 |
| accented, 4 KiB | 0.176 | 1.198 | 1.090 | 0.497 |
| accented, 1 MiB | 0.147 | 0.722 | 0.430 | 0.300 |
| binary, 64 B | 0.224 | 0.489 | 0.650 | 0.232 |
| binary, 4 KiB | 0.178 | 0.610 | 0.688 | 0.260 |
| binary, 1 MiB | 0.109 | 0.565 | 0.214 | 0.157 |

## Decoding

| input | before | quoted-printable-codec | encodify | quoted_printable | mail-parser |
| --- | --- | --- | --- | --- | --- |
| ascii, 64 B | 0.195 | 1.640 | 2.758 | 0.301 | 0.963 |
| ascii, 4 KiB | 0.163 | 3.186 | 2.600 | 0.592 | 1.334 |
| ascii, 1 MiB | 0.155 | 1.747 | 1.205 | 0.493 | 0.531 |
| accented, 64 B | 0.153 | 0.765 | 1.060 | 0.236 | 0.939 |
| accented, 4 KiB | 0.118 | 1.220 | 1.137 | 0.417 | 1.088 |
| accented, 1 MiB | 0.111 | 0.772 | 0.468 | 0.278 | 0.380 |
| binary, 64 B | 0.109 | 0.517 | 0.753 | 0.179 | 0.730 |
| binary, 4 KiB | 0.075 | 0.604 | 0.737 | 0.258 | 0.834 |
| binary, 1 MiB | 0.066 | 0.477 | 0.240 | 0.156 | 0.245 |

## Where it stands

2x to 20x faster than before, ahead of encodify in every 4 KiB and 1 MiB row, behind it in the six 64-byte rows (by 16% to 41%), and behind mail-parser decoding accented at 64 bytes and binary at 64 bytes and 4 KiB. A 64-byte call pays the fixed costs of three blocks; the 4 KiB rows of accented and binary are within the noise of the method (cycle counts from `perf` put encodify a few percent ahead there).

Zen 5 memorizes the branch history of an input repeated up to about 64 KiB, so no library mispredicts in the 64-byte and 4 KiB rows, which count instructions; in the 1 MiB rows encodify mispredicts 70 to 480 times a KiB, this crate 40 to 90, which is most of the gap there. Compare 1 MiB rows within one run.
