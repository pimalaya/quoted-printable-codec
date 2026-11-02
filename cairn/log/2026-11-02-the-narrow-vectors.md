---
cairn: log
change: the-narrow-vectors
landed: 2026-11-02
---

# The narrow vectors

On 2026-11-02 the user asked for the backends the handoff the-column-chain listed as missing, accuracy first and their speed left to later passes, and for Miri and fuzz targets. Before the work, they asked what the Q and DKIM modes of that handoff are. The answer given:

- Q is the RFC 2047 4.2 encoding of encoded words in header fields (`=?UTF-8?Q?Caf=C3=A9?=`): `_` stands for a space; space, tab, `?`, `=` and `_` are always encoded; a display name keeps only letters, digits and `!*+-/` literal (RFC 2047 5(3)); there are no line breaks, a word holding at most 75 characters, whole characters of its charset.
- DKIM is the DKIM-Quoted-Printable of RFC 6376 2.11, in DKIM-Signature tag values (`i=`, and `z=` with `|` encoded too): `;`, `=`, space, tab, CR and LF always encoded, no soft breaks, no line limit, whitespace ignored when decoding.
- The recommendation was to add neither. mime already holds its own Q encoder and decoder (mime/src/codec/quoted_printable.rs), encoded words are too short for SIMD to matter, and splitting text into words needs the charset knowledge mime holds; no pimalaya crate signs or verifies DKIM. The decision is the user's and still open.

What landed:

- The AVX2 backend, src/avx2.rs, for tokens holding AVX2, BMI1, LZCNT and POPCNT. Decoding takes the AVX-512 block steps on 32 bytes: the same masks, from `vpmovmskb`, hex digits by range compares, escapes decoded and blended in, the kept lanes compressed 8 at a time by `vpshufb` from `COMPRESS`, a table of 256 shuffles in lib.rs. Encoding takes chunks of 32 bytes in two groups of 16: the three bytes each byte may take, 0x80 where it takes none, are spread three lanes a byte by nine `vpshufb` a chunk from `SPREADS`; the kept lanes, found by their high bit, are compressed; `encode::line` places the soft break. Nothing takes `pext` or `pdep`, which Zen 2 runs in microcode.
- The NEON backend, src/neon.rs, on aarch64 builds with NEON: decoding in blocks of 16 bytes, encoding in chunks of 32 bytes as two groups of 16, a group spread by one `vqtbl3q_u8` a vector and compressed by `vtbl1_u8`, masks summed from weighted lanes by `vaddv_u8`.
- `encode::line`, shared by both: the soft break a group needs and the column after it, from the output offsets of its bytes, counted from the masks of escapes, line feeds and dropped CRs.
- `QuotedPrintableCpu::new` picks AVX-512, then AVX2, then portable on x86_64, and NEON, then portable on aarch64. `wide` is gone: `blocks` matches the backend, AVX-512 keeping its density check (`avx512::dense`), and runs each encoder through the `kernel!` macro over the four modes.
- Tests: `available()` lists the portable backend, the token without `Avx512Vbmi2` and the detected one. `every_backend_the_cpu_has_is_tested` fails where a backend the CPU has would go untested. Under Miri the bodies drop from 2,000 to 20 and the frozen corpus is skipped.
- fuzz/: the `encode` and `decode` cargo-fuzz targets and fuzz/README.md. Every backend the machine has is compared with the portable one, one-shot and resumed over random chunks and room; the portable encoding must decode back clean, in lines of at most 76 characters. fuzz/shell.nix is a nightly shell with Miri and cargo-fuzz, pinned through flake.lock.
- CI: an aarch64 job on `ubuntu-24.04-arm`, as base64-codec has, whose job passes on these private repositories; a Miri job (portable, `-C target-feature=+avx2,+bmi1,+lzcnt,+popcnt`, and `--target aarch64-unknown-linux-gnu`); short fuzz runs. None has run yet, as nothing is pushed.
- The README (features, a Testing section, the backend each CPU runs), benches/README.md, the crate docs, the specs, and the-column-chain, whose levers 2, 3 and 7 now describe the new backends as unmeasured.

Found on the way: the first NEON encoder took 16 bytes per check of `ROOM`, while the encoder's edge carries 32. Run over the carry into its 200-byte queue, it left up to 16 decided bytes for `finish`; every other backend leaves only a held space, tab or CR there. The bytes were right, but `finish` needed up to 51 bytes of room where the others need 9, and the scratch differential fuzzer, which retries `finish` with less than 30 bytes, looped. `finish_writes_the_held_bytes_only` failed on that encoder, under qemu at 22 bytes of text, and passes on every backend since NEON takes chunks of 32 bytes as AVX2 does.

Size: 786 lines before, 1,333 after (the counter of the passes before): avx2.rs 249, neon.rs 217, and 81 more in the shared files (the dispatch, `line`, `COMPRESS`, the backend choice). Speed was not measured, by the user's wish.

Checked:

- The suite on x86_64 under the portable, AVX2 and AVX-512 backends of this Zen 5: release and debug with all features, without default features, and release with `-C target-cpu=native`. The aarch64 suite under qemu-aarch64 on neoverse-n1, release and debug, built for aarch64-unknown-linux-musl.
- Each kernel broken on purpose fails the suite on its target: an escape's `=` in each encoder, the escape span in each decoder, the column in `line`, the second NEON group. Under Miri, the AVX2 and NEON breaks fail too.
- Miri: the library with the portable backend, with the AVX2 target features on the AVX2 kernels, and for aarch64-unknown-linux-gnu on the NEON kernels before and after the fix; the finish test, added after those runs, under the portable and AVX2 backends on x86_64.
- The scratch differential fuzzer of the-column-chain against the tree before's portable backend: 300,000 bodies on AVX2, and 2,000 on NEON under qemu after the fix.
- The fuzz targets on this machine's three backends: decode 4,466,570 runs in 10 minutes and encode 1,685,044 runs before the user stopped it, then 1,000,000 runs each on the final code, all clean.
- clippy with `-D warnings` for all targets on x86_64, for the library without default features, for aarch64-unknown-linux-musl, and for the fuzz crate; the library checked without warnings for riscv64gc-unknown-none-elf and aarch64-unknown-none-softfloat, targets with no SIMD backend; rustdoc with `-D warnings`; rustfmt.

Not run: the CI jobs, and NEON on aarch64 hardware.

Capabilities moved: backends (AVX2 and NEON, the widest backend, fuzzed inputs), calls (unsafe code in the SIMD backends).
