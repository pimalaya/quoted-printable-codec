---
cairn: log
change: the-q-encoding
landed: 2026-11-02
---

# The Q encoding

On 2026-11-02, told what the Q and DKIM modes of the handoff are, the user chose one codec over two to maintain: this crate takes the RFC 2047 4.2 Q encoding of encoded words, which mime holds in-house (mime/src/codec/quoted_printable.rs: `decode_q`, `encode_q`, `encoded_q_len`), so that mime can drop its copy. The DKIM-Quoted-Printable of RFC 6376 stays out, with no crate to use it.

mime's builder sizes each encoded word with `encoded_q_len` before writing it, and its parser leaves a word undecoded when `decode_q` reports it unclean (mime/src/codec/rfc2047.rs `parse_word`). The Q mode therefore copies mime's rules exactly, clean flag included: lowercase digits and raw bytes, common in words from the wild, stay clean, where this crate's body decoding reports both.

What landed:

- `QuotedPrintableMode`, `Text`, `Binary` or `Q`, in place of the codec's `binary` flag: `QuotedPrintableCodec { cpu, eol, mode }`. The tests, the bench and the fuzz targets moved to it; nothing outside the crate used the flag.
- Encoding in Q mode (src/encode.rs `words`): letters, digits and `!*+-/` literal, the phrase set of RFC 2047 5(3), which an encoded word may hold anywhere; a space as `_`; every other byte as `=XX`, uppercase; no line break of any kind, the line ending ignored. The caller splits text into words.
- Decoding in Q mode (src/decode.rs `words`): `_` as a space, `=XX` in either case, clean; a `=` starting no escape kept and reported, the bytes after decoded again; every other byte kept, clean. Unless the input ends, a `=` less than 2 bytes from the end of a chunk waits for the next one. `QuotedPrintableDecode` now holds the codec, not just its token, as decoding reads the mode.
- `QuotedPrintableCodec::encoded_len`, the exact length `encode` writes: a count in Q mode, the encoder run over a scratch buffer of twice `ROOM` in the body modes.
- Q runs these scalar steps under every token, as an encoded word holds 75 characters at most; the coroutines take Q codecs as they take the others.
- The AVX2 and NEON decoders took back `store` and `digit`, their helpers with a single caller.
- Docs: the crate root example now encodes an encoded word too; the README gains a feature line and the RFC 2047 row; the specs gain the Q requirements of both directions and the exact length, the body requirements marked as such.

Size: 1,404 lines (the counter of the passes before), from 1,333.

Checked:

- Against mime: a scratch crate ran 1,000,000 random inputs, rich in `=`, `_`, hex digits of both cases, blanks, line breaks and raw bytes, through this crate's Q mode under the portable and the detected backends and through mime's `encode_q`, `encoded_q_len` and `decode_q`: the same bytes, lengths and clean flags throughout.
- The suite, 28 tests, on x86_64 under the portable, AVX2 and AVX-512 backends, in release and debug with all features, without default features and in release with `-C target-cpu=native`, the frozen corpus keeping its hash; on aarch64 under qemu-aarch64 on neoverse-n1, release and debug. Q joins the split test in both directions, the length bound, one byte of room and the finish test; the decoder's one-byte-of-room test now drains what Q leaves queued before `finish`, which the body input never left.
- Each Q rule broken on purpose fails the suite: the space as `_`, `_` as a space, the escape width in `encoded_len`, the report of a broken escape.
- Miri on the library: portable, with the AVX2 target features, and for aarch64-unknown-linux-gnu with NEON, 25 tests each.
- The fuzz targets, the encode one choosing text, binary or Q and checking `encoded_len`, the decode one choosing a body or Q text: 1,000,000 runs each, clean.
- clippy with `-D warnings` for all targets on x86_64 and aarch64-unknown-linux-musl, for the library without default features, and for the fuzz crate; the library checked for riscv64gc-unknown-none-elf and aarch64-unknown-none-softfloat; rustdoc with `-D warnings`; rustfmt.

Not done: mime's switch to this crate, mime's own change, where its body decoding will also start reporting lowercase digits and raw bytes as this crate does.

Capabilities moved: encoding (the Q encoding), decoding (the Q encoding), calls (the mode, the exact encoded length).
