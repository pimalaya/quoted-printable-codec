# Benchmarks

[criterion](https://crates.io/crates/criterion) medians of `cargo bench --features alloc --bench compare` (source in [compare.rs](./compare.rs)), in GiB/s of raw bytes on both sides. Measured on 2026-11-01, rustc 1.99, Linux x86_64 on an AMD Ryzen AI Max+ 395 (Zen 5), pinned to one core with `taskset -c 13`, all groups in one run, on a machine shared with other work. Each cell of this tree is the median of three runs; "before" is one run of the tree of the pass before (commit e3f7726).

- quoted-printable-codec 0.0.1: blocks of 32 bytes walked by bitmask, safe code the compiler vectorizes (SSE2 here), the encoding walk compiled once per mode.
- encodify 1.0.2: `qp::BODY` (and `qp::BINARY` to encode binary), into a slice.
- quoted_printable 0.5.2: `encode`, `encode_binary` and `decode` in `ParseMode::Robust`; they allocate their output, as their API asks.
- mail-parser 0.11.9: `decoders::quoted_printable::quoted_printable_decode`, decoding only; it allocates its output.

Inputs: ascii is lowercase words with a few `.`, `,` and `=`, in CRLF lines of about 72 characters; accented is the same with about one character in seven a two-byte UTF-8 letter (é, è, à, ç, ê); binary is pseudo-random bytes, encoded in binary mode. Decoding reads quoted-printable-codec's encoding of each, and the bench checks that every decoder gives back the raw bytes.

## Encoding

| input | before | quoted-printable-codec | encodify | quoted_printable |
| --- | --- | --- | --- | --- |
| ascii, 64 B | 3.839 | 3.768 | 2.271 | 0.623 |
| ascii, 4 KiB | 2.847 | 3.993 | 2.241 | 0.735 |
| ascii, 1 MiB | 1.585 | 1.919 | 1.016 | 0.565 |
| ascii, 10 MiB | 1.826 | 1.723 | 0.944 | 0.530 |
| accented, 64 B | 1.219 | 1.394 | 0.939 | 0.395 |
| accented, 4 KiB | 1.348 | 1.606 | 1.047 | 0.444 |
| accented, 1 MiB | 0.764 | 0.923 | 0.381 | 0.297 |
| accented, 10 MiB | 0.811 | 0.945 | 0.405 | 0.296 |
| binary, 64 B | 0.743 | 1.208 | 0.626 | 0.234 |
| binary, 4 KiB | 0.825 | 1.328 | 0.675 | 0.257 |
| binary, 1 MiB | 0.746 | 0.857 | 0.212 | 0.155 |
| binary, 10 MiB | 0.698 | 0.913 | 0.213 | 0.138 |

## Decoding

| input | before | quoted-printable-codec | encodify | quoted_printable | mail-parser |
| --- | --- | --- | --- | --- | --- |
| ascii, 64 B | 3.456 | 2.492 | 2.292 | 0.261 | 1.190 |
| ascii, 4 KiB | 4.922 | 4.897 | 2.560 | 0.558 | 1.097 |
| ascii, 1 MiB | 1.902 | 1.881 | 1.151 | 0.475 | 0.473 |
| ascii, 10 MiB | 1.874 | 1.839 | 1.122 | 0.464 | 0.522 |
| accented, 64 B | 1.392 | 1.294 | 1.039 | 0.193 | 0.913 |
| accented, 4 KiB | 1.655 | 1.650 | 1.133 | 0.409 | 0.996 |
| accented, 1 MiB | 0.911 | 0.916 | 0.454 | 0.270 | 0.373 |
| accented, 10 MiB | 0.924 | 0.910 | 0.456 | 0.271 | 0.364 |
| binary, 64 B | 0.804 | 0.799 | 0.749 | 0.154 | 0.683 |
| binary, 4 KiB | 0.815 | 0.888 | 0.733 | 0.258 | 0.776 |
| binary, 1 MiB | 0.510 | 0.650 | 0.239 | 0.152 | 0.249 |
| binary, 10 MiB | 0.645 | 0.640 | 0.238 | 0.142 | 0.243 |

## Where it stands

Ahead of every other library in every row. Encoding is up to 1.75x faster than before in cycles (binary at 4 KiB from 5.6 to 3.2 cycles a byte); decoding is level with before, its changes this pass bought lines, not cycles. The closest row is decoding binary at 64 bytes, 6% ahead of encodify here and level with it by cycle counts.

criterion's 64-byte and 4 KiB cells move by up to 2x for this crate between runs of the same binary, with where the allocator places the buffers: decoding ascii at 64 bytes read 2.49, 1.79 and 3.24 GiB/s in the three runs, which is why its median reads below the one run before, while cycle counts from `perf` over eight buffer placements put it 10% faster. Compare cells closer than that with `perf` (cairn/changes/the-wide-blocks/proposal.md has the table and the method).

Zen 5 memorizes the branch history of an input repeated up to about 64 KiB, so no library mispredicts in the 64-byte and 4 KiB rows, which count instructions; in the 1 MiB and 10 MiB rows encodify mispredicts 70 to 480 times a KiB, this crate 33 to 85, which is most of the gap there.
