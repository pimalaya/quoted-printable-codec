# Benchmarks

[criterion](https://crates.io/crates/criterion) medians of `cargo bench --features alloc --bench compare` (source in [compare.rs](./compare.rs)), in GiB/s of raw bytes on both sides. Measured on 2026-11-01, rustc 1.99, Linux x86_64 on an AMD Ryzen AI Max+ 395 (Zen 5), pinned to one core with `taskset -c 13`, all groups in one run, on a machine shared with other work.

- quoted-printable-codec 0.0.1: blocks of 32 bytes walked by bitmask, safe code the compiler vectorizes (SSE2 here); "before" is the first block implementation (commit bb7e953), from the previous run of the same day.
- encodify 1.0.2: `qp::BODY` (and `qp::BINARY` to encode binary), into a slice.
- quoted_printable 0.5.2: `encode`, `encode_binary` and `decode` in `ParseMode::Robust`; they allocate their output, as their API asks.
- mail-parser 0.11.9: `decoders::quoted_printable::quoted_printable_decode`, decoding only; it allocates its output.

Inputs: ascii is lowercase words with a few `.`, `,` and `=`, in CRLF lines of about 72 characters; accented is the same with about one character in seven a two-byte UTF-8 letter (é, è, à, ç, ê); binary is pseudo-random bytes, encoded in binary mode. Decoding reads quoted-printable-codec's encoding of each, and the bench checks that every decoder gives back the raw bytes.

## Encoding

| input | before | quoted-printable-codec | encodify | quoted_printable |
| --- | --- | --- | --- | --- |
| ascii, 64 B | 1.702 | 3.670 | 2.299 | 0.621 |
| ascii, 4 KiB | 3.250 | 4.028 | 2.266 | 0.739 |
| ascii, 1 MiB | 1.685 | 1.956 | 1.016 | 0.574 |
| accented, 64 B | 0.829 | 1.382 | 0.979 | 0.363 |
| accented, 4 KiB | 1.198 | 1.505 | 1.073 | 0.484 |
| accented, 1 MiB | 0.722 | 0.807 | 0.404 | 0.305 |
| binary, 64 B | 0.489 | 0.665 | 0.637 | 0.243 |
| binary, 4 KiB | 0.610 | 0.766 | 0.681 | 0.258 |
| binary, 1 MiB | 0.565 | 0.706 | 0.211 | 0.156 |

## Decoding

| input | before | quoted-printable-codec | encodify | quoted_printable | mail-parser |
| --- | --- | --- | --- | --- | --- |
| ascii, 64 B | 1.640 | 2.820 | 2.759 | 0.262 | 0.957 |
| ascii, 4 KiB | 3.186 | 4.904 | 2.542 | 0.574 | 1.234 |
| ascii, 1 MiB | 1.747 | 1.939 | 1.156 | 0.484 | 0.520 |
| accented, 64 B | 0.765 | 1.402 | 1.045 | 0.199 | 0.923 |
| accented, 4 KiB | 1.220 | 1.657 | 1.115 | 0.396 | 1.052 |
| accented, 1 MiB | 0.772 | 0.908 | 0.444 | 0.267 | 0.368 |
| binary, 64 B | 0.517 | 0.785 | 0.735 | 0.151 | 0.718 |
| binary, 4 KiB | 0.604 | 0.899 | 0.733 | 0.255 | 0.822 |
| binary, 1 MiB | 0.477 | 0.652 | 0.234 | 0.157 | 0.252 |

## Where it stands

1.1x to 2.2x faster than before, and ahead of every other library in every row. The closest rows are decoding ascii and encoding binary at 64 bytes, 2% and 4% ahead of encodify here, within the run-to-run swing of criterion at that size (about 6%); cycle counts from `perf`, averaged over eight buffer offsets, put them 14% and 15% ahead.

Zen 5 memorizes the branch history of an input repeated up to about 64 KiB, so no library mispredicts in the 64-byte and 4 KiB rows, which count instructions; in the 1 MiB rows encodify mispredicts 70 to 480 times a KiB, this crate 35 to 90, which is most of the gap there. Compare 1 MiB rows within one run.
