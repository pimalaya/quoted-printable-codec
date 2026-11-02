---
cairn: spec
capability: encoding
status: current
---

# Encoding

Quoted-printable encoding of a MIME body (RFC 2045 6.7), in text or binary mode, with CRLF or LF line endings, and the Q encoding of the text of header words (RFC 2047 4.2), in Q mode. The line width is not a parameter: MIME fixes it, and Q writes none. Every requirement but the Q encoding is about the body modes.

### Requirement: Literal and escaped bytes

The encoder SHALL write `!` to `~` except `=` literally and every other byte as `=XX` with uppercase hex digits, except a space or tab not ending a line, written literally, and, in text mode, LF and CRLF, written as hard line breaks. A bare CR SHALL be escaped.

#### Scenario: Line-final blanks
- GIVEN `a \nb\t\r\nc ` in text mode
- WHEN it is encoded
- THEN the output is `a=20`, CRLF, `b=09`, CRLF, `c=20`

### Requirement: Binary mode

In binary mode, the encoder SHALL escape CR and LF like any other byte, so the output holds soft line breaks only.

### Requirement: Line length

The encoder SHALL insert a soft break (`=` and the line ending) before a literal or an escape that would take a line past 75 characters, so no line exceeds 76 with its `=`, and SHALL never split an escape.

#### Scenario: A long run
- GIVEN 80 bytes `x`
- WHEN they are encoded
- THEN the output is 75 `x`, `=`, CRLF, 5 `x`: 83 bytes

### Requirement: The line-start guard

With `guard`, in text and binary mode, the encoder SHALL escape a `.`, `F` or `-` that starts an output line, the first byte of the input or the first after a hard or soft break, so that no output line starts with one (RFC 2049 3(8)); a soft break SHALL count the three columns of a byte it pushes to a line start. Without `guard`, and in Q mode, the encoder SHALL write those bytes as any other.

#### Scenario: Markers at line starts
- GIVEN `.\nFrom x\n-- \n` in text mode with `guard`
- WHEN it is encoded with CRLF
- THEN the output is `=2E`, CRLF, `=46rom x`, CRLF, `=2D-=20`, CRLF

#### Scenario: A marker pushed to a line start
- GIVEN 75 bytes `a` then `F` then 10 bytes `a`, in text mode with `guard`
- WHEN it is encoded
- THEN the output is 75 `a`, `=`, CRLF, `=46`, 10 `a`, and no line exceeds 76 characters

### Requirement: The Q encoding

In Q mode, the encoder SHALL write letters, digits and `!*+-/` literally, a space as `_` and every other byte as `=XX` with uppercase hex digits, and no line break of any kind (RFC 2047 4.2 and 5(3)); the line ending is ignored.

#### Scenario: A phrase in Q
- GIVEN `a b,é?` in UTF-8
- WHEN it is encoded in Q mode
- THEN the output is `a_b=2C=C3=A9=3F`
