---
cairn: change
id: the-q-encoding
status: landed
created: 2026-11-02
---

# The Q encoding

## Why

On 2026-11-02 the user chose to have one codec rather than two to maintain: this crate takes the RFC 2047 4.2 Q encoding of encoded words, which mime holds in-house today (mime/src/codec/quoted_printable.rs: `decode_q`, `encode_q`, `encoded_q_len`), so that mime can drop its copy. The DKIM-Quoted-Printable of RFC 6376 stays out: no crate has a use for it.

mime's builder sizes each encoded word with `encoded_q_len` before writing it, and its parser leaves a word undecoded when `decode_q` reports it unclean. So the Q mode copies mime's rules exactly, clean flag included: lowercase digits and raw bytes are common in the wild, and reporting them would leave words undecoded that mime decodes today.

## What

- `QuotedPrintableMode`, `Text`, `Binary` or `Q`, replaces the codec's `binary` flag: `QuotedPrintableCodec { cpu, eol, mode }`.
- Encoding in Q mode: letters, digits and `!*+-/` literal (the phrase set of RFC 2047 5(3), safe wherever an encoded word goes), a space as `_`, every other byte as `=XX` with uppercase digits, no line break of any kind; the line ending is ignored, and the caller splits text into encoded words.
- Decoding in Q mode: `_` as a space, `=XX` in either case, clean; a `=` starting no escape kept and reported, the bytes after decoded again; every other byte kept as it is, clean. The decoder now reads the codec's mode, Q or not.
- `QuotedPrintableCodec::encoded_len`, the exact length `encode` writes: a count in Q mode, the encoder run into a scratch buffer in the others.
- Q runs the portable steps on every backend: an encoded word holds 75 characters at most, so SIMD would buy nothing.
- The coroutines take Q codecs like the others: nothing is held between calls but an escape cut short when decoding.
- Tests, the fuzz targets and the bench move to the mode; the README gains the RFC 2047 row.

Out of scope: switching mime to this crate, which is mime's own change.
