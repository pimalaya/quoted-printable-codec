---
cairn: delta
change: the-q-encoding
---

## ADDED Requirements

### Requirement: The Q encoding

In Q mode, the encoder SHALL write letters, digits and `!*+-/` literally, a space as `_` and every other byte as `=XX` with uppercase hex digits, and no line break of any kind (RFC 2047 4.2 and 5(3)); the line ending is ignored.

#### Scenario: A phrase in Q
- GIVEN `a b,é?` in UTF-8
- WHEN it is encoded in Q mode
- THEN the output is `a_b=2C=C3=A9=3F`

### Requirement: Decoding the Q encoding

In Q mode, the decoder SHALL read `_` as a space and `=XX` in either case of hex digits, clean; it SHALL keep a `=` that starts no escape and report it, the bytes after decoded again; and it SHALL keep every other byte as it is, clean, line breaks and blanks included.

#### Scenario: Words from the wild
- GIVEN `caf=c3=a9_au_lait`, then `a=zb`
- WHEN each is decoded in Q mode
- THEN the outputs are `café au lait`, clean, and `a=zb`, unclean

### Requirement: The exact encoded length

`QuotedPrintableCodec::encoded_len` SHALL give the exact number of bytes `QuotedPrintableCodec::encode` writes for an input, in every mode.

## MODIFIED Requirements

### Requirement: One codec for every call

Every call SHALL run from a `QuotedPrintableCodec`, a `Copy` value holding the backend token, the mode (text, binary or Q) and the line ending of encoded output: its one-shot methods encode or decode a whole input, and `QuotedPrintableEncode::new` and `QuotedPrintableDecode::new` take it to encode or decode chunk by chunk. The codec SHALL hold no state between calls, so that no one-shot call can run in the middle of a resumed input. Decoding SHALL take any input whatever the codec's line ending, as a MIME body in text and binary modes and as the text of encoded words in Q mode.

#### Scenario: One codec for the whole application
- GIVEN a codec built once in binary mode with LF line endings
- WHEN it encodes an input in one call, and a `QuotedPrintableEncode` built from it encodes the same input chunk by chunk, then finishes
- THEN both write the same bytes
