---
cairn: spec
capability: decoding
status: current
---

# Decoding

Lenient quoted-printable decoding of a MIME body, as RFC 2045 6.7 asks of robust implementations. Decoding never fails: it returns the bytes and whether the input was clean.

### Requirement: Escapes and soft breaks

The decoder SHALL turn `=XX` into its byte, either case of hex digits accepted and lowercase reported, and SHALL join a soft break: `=`, optional spaces or tabs, then CRLF, LF or the end of the input.

### Requirement: Line breaks and trailing blanks

The decoder SHALL keep CRLF and bare LF as they are, drop a bare CR and report it, and delete the spaces and tabs that end a line or the input. It SHALL carry at most 76 such blanks: past them, the oldest are kept as data.

#### Scenario: Too many blanks
- GIVEN 78 spaces then CRLF
- WHEN they are decoded
- THEN the output is 2 spaces then CRLF, reported clean

### Requirement: Malformed input

A `=` that starts neither an escape nor a soft break SHALL be kept with the hex digit after it, if any, and reported; the bytes after are decoded again as data. A byte outside `!` to `~`, space, tab, CR and LF SHALL be kept and reported.

#### Scenario: Malformed escapes
- GIVEN `a=zb`, `a=4zb` and `a==41`
- WHEN each is decoded
- THEN the outputs are `a=zb`, `a=4zb` and `a=A`, each reported unclean
