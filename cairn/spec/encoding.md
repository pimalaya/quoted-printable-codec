---
cairn: spec
capability: encoding
status: current
---

# Encoding

Quoted-printable encoding of a MIME body (RFC 2045 6.7), in text or binary mode, with CRLF or LF line endings. The line width is not a parameter: MIME fixes it.

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
