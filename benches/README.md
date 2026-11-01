# Benchmarks

[criterion](https://crates.io/crates/criterion) medians of `cargo bench --features alloc --bench compare` (source in [compare.rs](./compare.rs)), in GiB/s of raw bytes on both sides. Measured on 2026-11-01, rustc 1.99, Linux x86_64 on an AMD Ryzen AI Max+ 395 (Zen 5), pinned to one core with `taskset -c 13`, all groups in one run, on a machine shared with other work.

- quoted-printable-codec 0.0.1: the scalar byte-by-byte implementation, no SIMD.
- encodify 1.0.2: `qp::BODY` (and `qp::BINARY` to encode binary), into a slice.
- quoted_printable 0.5.2: `encode`, `encode_binary` and `decode` in `ParseMode::Robust`; they allocate their output, as their API asks.
- mail-parser 0.11.9: `decoders::quoted_printable::quoted_printable_decode`, decoding only; it allocates its output.

Inputs: ascii is lowercase words with a few `.`, `,` and `=`, in CRLF lines of about 72 characters; accented is the same with about one character in seven a two-byte UTF-8 letter (é, è, à, ç, ê); binary is pseudo-random bytes, encoded in binary mode. Decoding reads quoted-printable-codec's encoding of each, and the bench checks that every decoder gives back the raw bytes.

## Encoding

| input | quoted-printable-codec | encodify | quoted_printable |
| --- | --- | --- | --- |
| ascii, 64 B | 0.215 | 2.332 | 0.644 |
| ascii, 4 KiB | 0.191 | 2.283 | 0.755 |
| ascii, 1 MiB | 0.178 | 1.091 | 0.587 |
| accented, 64 B | 0.231 | 0.986 | 0.376 |
| accented, 4 KiB | 0.176 | 1.089 | 0.495 |
| accented, 1 MiB | 0.147 | 0.438 | 0.315 |
| binary, 64 B | 0.224 | 0.648 | 0.236 |
| binary, 4 KiB | 0.178 | 0.688 | 0.263 |
| binary, 1 MiB | 0.109 | 0.200 | 0.157 |

## Decoding

| input | quoted-printable-codec | encodify | quoted_printable | mail-parser |
| --- | --- | --- | --- | --- |
| ascii, 64 B | 0.195 | 2.770 | 0.301 | 1.254 |
| ascii, 4 KiB | 0.163 | 2.609 | 0.600 | 1.312 |
| ascii, 1 MiB | 0.155 | 1.196 | 0.488 | 0.529 |
| accented, 64 B | 0.153 | 1.069 | 0.226 | 0.929 |
| accented, 4 KiB | 0.118 | 1.143 | 0.413 | 1.006 |
| accented, 1 MiB | 0.111 | 0.469 | 0.276 | 0.379 |
| binary, 64 B | 0.109 | 0.744 | 0.151 | 0.660 |
| binary, 4 KiB | 0.075 | 0.734 | 0.259 | 0.764 |
| binary, 1 MiB | 0.066 | 0.241 | 0.155 | 0.245 |

## Where it stands

Last everywhere: encodify is 3x to 16x ahead on text and 1.8x to 10x on binary, the quoted_printable crate 1.05x to 4x despite allocating, mail-parser 3x to 10x. The cost is per byte and flat across sizes: one state-machine step, one push into the output queue and one drain per input byte, about 0.1 to 0.2 GiB/s whatever the content. The first levers are copying runs of literal bytes straight into the output (encodify's `copy_plain` and `copy_text`) and writing into the caller's buffer directly once it has room for a whole step, before any SIMD.

Every library slows at 1 MiB (encodify's ascii decode from 2.6 to 1.2 GiB/s); this run did not tell whether that is the shared machine or the working set leaving the cache, so compare 1 MiB rows only within one run.
