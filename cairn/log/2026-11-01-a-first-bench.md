---
cairn: log
change: a-first-bench
landed: 2026-11-01
---

# A first bench

Added benches/compare.rs against encodify 1.0.2, quoted_printable 0.5.2 and mail-parser 0.11.9 on ascii, accented and binary bodies from 64 bytes to 1 MiB, every decoder checked to give back the raw bytes; numbers in benches/README.md. The scalar byte-by-byte implementation runs at 0.07 to 0.23 GiB/s, last in every row: encodify is 3x to 16x ahead on text. The first levers are copying literal runs straight into the output and writing into the caller's buffer directly when it has room, before any SIMD.

Capabilities moved: none.
