---
cairn: spec
capability: decoding
status: current
---

# Decoding

Lenient quoted-printable decoding of a MIME body, as RFC 2045 6.7 asks of robust implementations, and of the Q encoding of header words in Q mode (RFC 2047 4.2). Decoding never fails: it returns the bytes and whether the input was clean. Every requirement but the Q one is about the body modes, text and binary.

### Requirement: Escapes and soft breaks

The decoder SHALL turn `=XX` into its byte, either case of hex digits accepted and lowercase reported, and SHALL join a soft break: `=`, optional spaces or tabs, then CRLF, LF or the end of the input.

### Requirement: Line breaks and trailing blanks

The decoder SHALL keep CRLF and bare LF as they are, keep a bare CR as data and report it, and delete the spaces and tabs that end a line or the input. It SHALL carry at most 76 such blanks: past them, the oldest are kept as data.

#### Scenario: Too many blanks
- GIVEN 78 spaces then CRLF
- WHEN they are decoded
- THEN the output is 2 spaces then CRLF, reported clean

#### Scenario: Lines ended by CR alone
- GIVEN `a\rb\r`
- WHEN it is decoded
- THEN the output is `a\rb\r`, reported unclean

### Requirement: Malformed input

A `=` that starts neither an escape nor a soft break SHALL be kept with the hex digit after it, if any, and reported; the bytes after are decoded again as data. A byte outside `!` to `~`, space, tab, CR and LF SHALL be kept and reported.

#### Scenario: Malformed escapes
- GIVEN `a=zb`, `a=4zb` and `a==41`
- WHEN each is decoded
- THEN the outputs are `a=zb`, `a=4zb` and `a=A`, each reported unclean

### Requirement: Decoding the Q encoding

In Q mode, the decoder SHALL read `_` as a space and `=XX` in either case of hex digits, clean; it SHALL keep a `=` that starts no escape and report it, the bytes after decoded again; and it SHALL keep every other byte as it is, clean, line breaks and blanks included.

#### Scenario: Words from the wild
- GIVEN `caf=c3=a9_au_lait`, then `a=zb`
- WHEN each is decoded in Q mode
- THEN the outputs are `café au lait`, clean, and `a=zb`, unclean
