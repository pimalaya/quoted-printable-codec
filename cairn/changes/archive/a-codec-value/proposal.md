---
cairn: change
id: a-codec-value
status: landed
created: 2026-11-01
---

# A codec value

## Why

The one-shot functions stood apart from the encoder and decoder: free functions a reader of `QuotedPrintableEncode` never saw on docs.rs, called as `encode::encode`, taking the options `QuotedPrintableEncode::new` takes too. base64-codec moved to a codec value the same day (its a-codec-value); the user asked for the same shape here, so both crates read alike.

## What

`codec::QuotedPrintableCodec`, a plain `Copy` value holding the line ending (`eol`) and the mode (`binary`), defaulting to text with CRLF, with the one-shot methods `encode`, `encode_vec`, `encoded_len_max`, `decode`, `decode_vec` and `decoded_len_max`. `QuotedPrintableEncode::new` and `QuotedPrintableDecode::new` take a codec, the decoder ignoring it as decoding takes any input. `QuotedPrintableEncodeOptions` and the free functions go, `QuotedPrintableEol` moves to the codec module. Resuming and finishing do not change.

There is no `BASELINE` constant as base64-codec has: there is no CPU token to pick, and the fields are public, so a `static` writes the struct out.
