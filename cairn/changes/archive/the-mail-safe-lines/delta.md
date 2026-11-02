---
cairn: delta
change: the-mail-safe-lines
---

## ADDED Requirements

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

## MODIFIED Requirements

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

## REMOVED Requirements
