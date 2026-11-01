---
cairn: delta
change: the-lean-pass
---

## MODIFIED Requirements

### Requirement: Lengths and short buffers

`QuotedPrintableCodec::encoded_len_max` SHALL give the room `QuotedPrintableCodec::encode` needs, a bound on what it writes plus the 160 bytes its blocks may touch past the output, or return none when that overflows `usize`; `QuotedPrintableCodec::decoded_len_max` SHALL bound what `QuotedPrintableCodec::decode` writes. `encode` and `decode` SHALL return none and write nothing when `dst` is shorter than their length helper asks. Every call MAY overwrite the bytes of `dst` past the ones it reports written, within the length it was given.

### Requirement: Resumable without I/O

`QuotedPrintableEncode` and `QuotedPrintableDecode` SHALL give, for any split of the input into chunks and any output space offered, the same bytes and the same clean flag as the one-shot call, `finish` ending the input. `resume` SHALL always progress with one byte of room, and `finish` SHALL write nothing and return none when its output does not fit. A `finish` that succeeds SHALL leave the coroutine as new, ready for another input.

#### Scenario: A coroutine reused after finish
- GIVEN an encoder that encoded and finished one input, ending mid-line, and a decoder that decoded and finished an unclean input
- WHEN each takes another input and finishes again
- THEN each gives what a new one gives: the encoder breaks lines from column 0, and the decoder reports the second input clean when it is
