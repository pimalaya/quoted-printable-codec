---
cairn: tasks
change: a-codec-value
---

- [x] `codec::QuotedPrintableCodec` with `eol` and `binary`, and `QuotedPrintableEol` moved next to it
- [x] The one-shot methods on the codec, the free functions and `QuotedPrintableEncodeOptions` removed
- [x] `QuotedPrintableEncode::new` and `QuotedPrintableDecode::new` taking a codec
- [x] Docs, tests and the bench on the new calls
- [x] Output unchanged (`a_frozen_corpus_keeps_its_output`), criterion at 4 KiB within noise
- [x] Spec folded, log entry written
