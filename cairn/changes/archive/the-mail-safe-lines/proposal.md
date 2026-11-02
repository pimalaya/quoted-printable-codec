---
cairn: change
id: the-mail-safe-lines
status: landed
created: 2026-11-02
---

# The mail-safe lines

## Why

mime-rs moves its quoted-printable body codec to this crate (its change quoted-printable-over-the-codec), and two of its rules have no counterpart here. The user settled both on 2026-11-02, in favour of mime's rules, and on review kept the guard an option, off by default, for now:

- **A line-start guard.** mime escapes a `.`, `F` or `-` that would start an encoded line. RFC 2049 3(8) recommends it: a lone `.` ends an SMTP DATA body, `From ` at a line start is rewritten by mbox writers, and a line starting `--` can read as a MIME delimiter. mime's builder relies on the last one: it treats quoted-printable as always safe inside a multipart, since no line of its output can be taken for a delimiter of a declared or enclosing boundary.
- **A bare CR kept.** This decoder drops a bare CR and reports it, where mime keeps it. A body whose lines end with CR alone, as old Mac mail has them, would lose every line break.

## What

- **Encoding.** A `guard: bool` on `QuotedPrintableCodec`, off by default, so the codec's output does not change for anyone who does not ask. In text and binary mode, with `guard`, a `.`, `F` or `-` that starts an output line (the first byte of the input, or the first byte after a hard or soft break) is escaped as `=2E`, `=46` or `=2D`. Soft breaks account for its three columns: a byte pushed to the start of a line by a soft break is escaped, and the break is placed so no line exceeds 76 characters. Q mode ignores `guard`: an encoded word starts no line. mime escapes `F` whatever follows it rather than only before `rom `, which keeps the rule a property of one byte: the same here.
- **Decoding.** A bare CR, one not before an LF, is kept as data and reported, in every mode but Q, where no line break means anything and the byte is kept as data and clean, as Q keeps every other byte.
- **Backends.** Every backend keeps writing the bytes and flag of the portable one. The portable walk, built per guard, takes a guarded byte as an event of width 1, escaped when it starts a line. The SIMD kernels, built per guard too so a codec without `guard` runs kernels without the check, find the line starts of a group (column 0, after a line feed, at the soft break) and stop before a group where a guarded byte would sit at one; the portable walk then encodes the next 48 bytes and the kernel resumes. The decoding kernels already single out CR for the token steps; keeping it instead of dropping it changes the token step alone.
- **Tests, fuzz, bench.** The frozen corpus gains `guard` codecs (its hash moves for decoding, since a bare CR is now kept); the fuzz targets draw `guard`; the fuzz oracle that encoded output decodes back gains: with `guard`, no output line starts with `.`, `F` or `-`. The bench compares HEAD and the tree on every row without `guard`, which must not move, and adds guarded text rows.

Out of scope, left to mime's change: the two smaller differences between the two decoders, the blanks past 76 before a line break (kept as data here, dropped by mime) and a `=` followed by blanks at the end of the input (a clean soft break here, reported by mime).

## Risks

- The guard adds a column-0 case to the soft-break placement every backend computes; handed to the portable walk, it costs only lines that start with a guarded byte, which text rarely has, and a text where most lines do (a list of `-` items) runs closer to the portable speed.
- Keeping a bare CR changes the decoding of inputs that had one: the frozen hash moves on purpose, and the log says so.
