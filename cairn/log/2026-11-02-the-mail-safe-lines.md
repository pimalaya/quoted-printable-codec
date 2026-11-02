---
cairn: log
change: the-mail-safe-lines
landed: 2026-11-02
---

# The mail-safe lines

Landed the two rules mime-rs needs before it moves its quoted-printable codec here (its change quoted-printable-over-the-codec). The user asked on review what the guard is for, and chose to keep it an option, off by default, for now.

- `QuotedPrintableCodec::guard`: in text and binary mode, a `.`, `F` or `-` that starts an output line is escaped (RFC 2049 3(8)), soft breaks counting its three columns. Q mode ignores it. `BASELINE` and `default()` leave it off, so unguarded output did not move: a 3,000-body corpus with `.`, `F` and `-` encoded and decoded back byte-identical to HEAD, on the portable and AVX-512 backends.
- Decoding keeps a bare CR as data and reports it, where it dropped it; the blanks before it are data too. Q mode never reported a CR and still does not.
- The portable walk is built per guard: a guarded byte is an event of width 1, escaped when its output position is the line's start. The SIMD encoders are built per guard too (eight monomorphizations through `kernel!`), so an unguarded codec runs kernels without the check. A guarded group finds its line starts (column 0, the byte after a line feed, the token at the soft break: `line()` for AVX2 and NEON in input space, the AVX-512 group in output space after its `pext`) and the kernel stops before the group when a guarded byte sits at one, returning 1 as its third count. `encode::chain` then runs the portable walk over the next 48 bytes and resumes the kernel from the walk's column.

Verification: the suite on x86_64 in debug and release, all features and none; every backend against the portable one over 2,000 random bodies, now drawing `.` and `-` too, with the eight body codecs; a new oracle that no guarded line starts with a guarded byte and that guarded binary decodes back; mime's cases (a marker pushed to a line start at every column from 60 to 160 in every mode and line ending, a blank before a lone CR from 60 to 80). Removing the soft-break start from `line()`, or the column-0 or soft-break start from the AVX-512 group, fails four or five tests each. The frozen corpus hash moved to 10479309502388201493 (guarded codecs and the kept CR). The encode fuzz target draws `guard` and checks line starts: 230,000 runs, and 163,000 of decode, clean. Miri ran the suite on aarch64 through the NEON kernels clean (29 tests, the frozen corpus skipped as before).

Bench (criterion medians, Zen 5, AVX-512 backend): every unguarded row within 5% of HEAD in both directions, the machine's noise. Guarded text encodes ascii 64 B in 27 ns (17 unguarded), 4 KiB in 951 ns (676), 1 MiB in 229 us (168), 10 MiB in 2.85 ms (1.68); accented 1 MiB in 232 us (162). About 7% of the bench's lines start with `.` or `F`, each costing a kernel stop and a 48-byte walk; a text with fewer guarded line starts runs closer to the unguarded speed.

Capabilities moved: encoding (ADDED: The line-start guard), decoding (MODIFIED: Line breaks and trailing blanks), backends (the guarded kernels, every backend matching the portable one per guard).
