---
cairn: tasks
change: the-mail-safe-lines
---

- [x] `QuotedPrintableCodec::guard`, off by default and in `BASELINE`; the portable encoder escapes a guarded byte at a line start, soft breaks counting its three columns
- [x] The AVX-512, AVX2 and NEON encoders, built per guard, stop before a group where a guarded byte would start a line and leave the next 48 bytes to the portable walk
- [x] Decoding keeps a bare CR and reports it, in every backend; Q keeps it clean
- [x] Tests: the frozen corpus with `guard` codecs and `.` and `-` in its bodies, its new hash; mime's guard cases (a marker pushed to a line start at every column from 60 to 160, a blank before a lone CR at every column from 60 to 80); the no-guarded-line-start oracle over every backend
- [x] Fuzz target drawing `guard`, with the no-guarded-line-start oracle; Miri on the portable backend and the SIMD kernels
- [x] Bench: every row without `guard` against HEAD; guarded text rows added
- [x] README, spec folded, log entry, change archived
