# 🔣 quoted-printable-codec [![Matrix](https://img.shields.io/badge/chat-%23pimalaya-blue?style=flat&logo=matrix&logoColor=white)](https://matrix.to/#/#pimalaya:matrix.org) [![Mastodon](https://img.shields.io/badge/news-%40pimalaya-blue?style=flat&logo=mastodon&logoColor=white)](https://fosstodon.org/@pimalaya) [![Sponsor](https://img.shields.io/badge/sponsor-pink?style=flat&logo=github-sponsors&logoColor=white)](https://pimalaya.org/sponsor/)

Quoted-printable encoder and decoder, no_std and dependency-free

## Table of contents

- [Features](#features)
- [RFC coverage](#rfc-coverage)
- [Usage](#usage)
- [Examples](#examples)
- [Benchmarks](#benchmarks)
- [AI policy](https://github.com/pimalaya/.github/blob/master/AI_POLICY.md)
- [License](#license)
- [Social](#social)
- [Sponsoring](#sponsoring)

## Features

- **Fast**: up to 4x faster than encodify and 9x faster than the quoted_printable crate, see the [benchmarks](#benchmarks).
- **MIME lines**: encodes in lines of at most 76 characters, soft breaks included, ended by CRLF or LF.
- **Text and binary**: text mode keeps line breaks as hard breaks, binary mode escapes CR and LF like any other byte.
- **Lenient decoding**: never fails, follows the robust rules of RFC 2045 (lowercase digits, trailing blanks, bare CRs, malformed escapes kept), and reports whether the input was clean.
- **I/O-free**: encode and decode chunk by chunk into your own buffers.
- **Portable**: no_std, dependency-free, no unsafe code; the compiler vectorizes the byte scans for any target, with no CPU feature beyond its baseline.

## RFC coverage

| RFC    | What is covered                                                                                                        |
|--------|------------------------------------------------------------------------------------------------------------------------|
| [2045] | The quoted-printable content transfer encoding (6.7): escapes, soft line breaks, lines of 76 characters, robust decoding |

[2045]: https://www.rfc-editor.org/rfc/rfc2045#section-6.7

## Usage

The whole API is documented on [docs.rs](https://docs.rs/quoted-printable-codec/latest/quoted_printable_codec), with runnable snippets for the codec and its one-shot methods, the encoder and the decoder.

## Examples

The crate ships no examples folder, a whole program adding nothing to a two-call API. The crate root, the encoder and the decoder each open with a runnable snippet.

## Benchmarks

[criterion](https://crates.io/crates/criterion) medians of `cargo bench --features alloc --bench compare` on 2026-11-01, on an AMD Ryzen AI Max+ 395 (Zen 5), in GiB/s of raw bytes. Each italic delta compares its row with quoted-printable-codec.

### ASCII decoding

| GiB/s | 64 B | 4 KiB | 1 MiB | 10 MiB |
| --- | ---: | ---: | ---: | ---: |
| **quoted-printable-codec** | **2.49** | **4.90** | **1.88** | **1.84** |
| [encodify](https://crates.io/crates/encodify) | *(-8%)* **2.29** | *(-48%)* **2.56** | *(-39%)* **1.15** | *(-39%)* **1.12** |
| [quoted_printable](https://crates.io/crates/quoted_printable) | *(-90%)* **0.26** | *(-89%)* **0.56** | *(-75%)* **0.47** | *(-75%)* **0.46** |
| [mail-parser](https://crates.io/crates/mail-parser) | *(-52%)* **1.19** | *(-78%)* **1.10** | *(-75%)* **0.47** | *(-72%)* **0.52** |

### Accented decoding

| GiB/s | 64 B | 4 KiB | 1 MiB | 10 MiB |
| --- | ---: | ---: | ---: | ---: |
| **quoted-printable-codec** | **1.29** | **1.65** | **0.92** | **0.91** |
| [encodify](https://crates.io/crates/encodify) | *(-20%)* **1.04** | *(-31%)* **1.13** | *(-50%)* **0.45** | *(-50%)* **0.46** |
| [quoted_printable](https://crates.io/crates/quoted_printable) | *(-85%)* **0.19** | *(-75%)* **0.41** | *(-71%)* **0.27** | *(-70%)* **0.27** |
| [mail-parser](https://crates.io/crates/mail-parser) | *(-29%)* **0.91** | *(-40%)* **1.00** | *(-59%)* **0.37** | *(-60%)* **0.36** |

### Binary decoding

| GiB/s | 64 B | 4 KiB | 1 MiB | 10 MiB |
| --- | ---: | ---: | ---: | ---: |
| **quoted-printable-codec** | **0.80** | **0.89** | **0.65** | **0.64** |
| [encodify](https://crates.io/crates/encodify) | *(-6%)* **0.75** | *(-17%)* **0.73** | *(-63%)* **0.24** | *(-63%)* **0.24** |
| [quoted_printable](https://crates.io/crates/quoted_printable) | *(-81%)* **0.15** | *(-71%)* **0.26** | *(-77%)* **0.15** | *(-78%)* **0.14** |
| [mail-parser](https://crates.io/crates/mail-parser) | *(-15%)* **0.68** | *(-13%)* **0.78** | *(-62%)* **0.25** | *(-62%)* **0.24** |

### ASCII encoding

| GiB/s | 64 B | 4 KiB | 1 MiB | 10 MiB |
| --- | ---: | ---: | ---: | ---: |
| **quoted-printable-codec** | **3.77** | **3.99** | **1.92** | **1.72** |
| [encodify](https://crates.io/crates/encodify) | *(-40%)* **2.27** | *(-44%)* **2.24** | *(-47%)* **1.02** | *(-45%)* **0.94** |
| [quoted_printable](https://crates.io/crates/quoted_printable) | *(-83%)* **0.62** | *(-82%)* **0.73** | *(-71%)* **0.56** | *(-69%)* **0.53** |

### Accented encoding

| GiB/s | 64 B | 4 KiB | 1 MiB | 10 MiB |
| --- | ---: | ---: | ---: | ---: |
| **quoted-printable-codec** | **1.39** | **1.61** | **0.92** | **0.94** |
| [encodify](https://crates.io/crates/encodify) | *(-33%)* **0.94** | *(-35%)* **1.05** | *(-59%)* **0.38** | *(-57%)* **0.41** |
| [quoted_printable](https://crates.io/crates/quoted_printable) | *(-72%)* **0.40** | *(-72%)* **0.44** | *(-68%)* **0.30** | *(-69%)* **0.30** |

### Binary encoding

| GiB/s | 64 B | 4 KiB | 1 MiB | 10 MiB |
| --- | ---: | ---: | ---: | ---: |
| **quoted-printable-codec** | **1.21** | **1.33** | **0.86** | **0.91** |
| [encodify](https://crates.io/crates/encodify) | *(-48%)* **0.63** | *(-49%)* **0.68** | *(-75%)* **0.21** | *(-77%)* **0.21** |
| [quoted_printable](https://crates.io/crates/quoted_printable) | *(-81%)* **0.23** | *(-81%)* **0.26** | *(-82%)* **0.15** | *(-85%)* **0.14** |

### Reading the numbers

- quoted-printable-codec runs the same safe code on every CPU; here the compiler vectorizes its byte scans with SSE2, the x86_64 baseline.
- ASCII is lowercase words with a few `.`, `,` and `=` in CRLF lines of about 72 characters; accented is the same with about one character in seven a two-byte UTF-8 letter; binary is pseudo-random bytes, encoded in binary mode. Decoding reads quoted-printable-codec's encoding of each, and every decoder gives back the raw bytes.
- quoted_printable and mail-parser allocate their output, as their APIs ask; mail-parser only decodes.
- Each cell is the median of three runs: quoted-printable-codec's 64 B and 4 KiB cells move by up to 2x between runs with where the allocator places the buffers (decoding ASCII at 64 B read 1.79, 2.49 and 3.24 GiB/s), the memory-bound 1 MiB and 10 MiB ones by up to 40%.

History and cycle counts are in [benches/README.md](./benches/README.md).

## License

This project is licensed under either of:

- [MIT license](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

## Social

- Chat on [Matrix](https://matrix.to/#/#pimalaya:matrix.org)
- News on [Mastodon](https://fosstodon.org/@pimalaya) or [RSS](https://fosstodon.org/@pimalaya.rss)
- Mail at [pimalaya.org@posteo.net](mailto:pimalaya.org@posteo.net)

## Sponsoring

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/)

Special thanks to the [NLnet foundation](https://nlnet.nl/) and the [European Commission](https://www.ngi.eu/) that have been financially supporting the project for years:

- 2022 → 2023: [NGI Assure](https://nlnet.nl/project/Himalaya/)
- 2023 → 2024: [NGI Zero Entrust](https://nlnet.nl/project/Pimalaya/)
- 2024 → 2026: [NGI Zero Core](https://nlnet.nl/project/Pimalaya-PIM/)
- 2026 → 2027: [NGI Zero Commons Fund](https://nlnet.nl/project/Pimalaya-pimdir/)

This program is part of Pimalaya, free software funded entirely by grants and donations. If you find it useful, consider [sponsoring](https://pimalaya.org/sponsor/) its development:

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/pimalaya)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/pimalaya)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/pimalaya)
[![thanks.dev](https://img.shields.io/badge/-thanks.dev-000000?logo=data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQuMDk3IiBoZWlnaHQ9IjE3LjU5NyIgY2xhc3M9InctMzYgbWwtMiBsZzpteC0wIHByaW50Om14LTAgcHJpbnQ6aW52ZXJ0IiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxwYXRoIGQ9Ik05Ljc4MyAxNy41OTdINy4zOThjLTEuMTY4IDAtMi4wOTItLjI5Ny0yLjc3My0uODktLjY4LS41OTMtMS4wMi0xLjQ2Mi0xLjAyLTIuNjA2di0xLjM0NmMwLTEuMDE4LS4yMjctMS43NS0uNjc4LTIuMTk1LS40NTItLjQ0Ni0xLjIzMi0uNjY5LTIuMzQtLjY2OUgwVjcuNzA1aC41ODdjMS4xMDggMCAxLjg4OC0uMjIyIDIuMzQtLjY2OC40NTEtLjQ0Ni42NzctMS4xNzcuNjc3LTIuMTk1VjMuNDk2YzAtMS4xNDQuMzQtMi4wMTMgMS4wMjEtMi42MDZDNS4zMDUuMjk3IDYuMjMgMCA3LjM5OCAwaDIuMzg1djEuOTg3aC0uOTg1Yy0uMzYxIDAtLjY4OC4wMjctLjk4LjA4MmExLjcxOSAxLjcxOSAwIDAgMC0uNzM2LjMwN2MtLjIwNS4xNTYtLjM1OC4zODQtLjQ2LjY4Mi0uMTAzLjI5OC0uMTU0LjY4Mi0uMTU0IDEuMTUxVjUuMjNjMCAuODY3LS4yNDkgMS41ODYtLjc0NSAyLjE1NS0uNDk3LjU2OS0xLjE1OCAxLjAwNC0xLjk4MyAxLjMwNXYuMjE3Yy44MjUuMyAxLjQ4Ni43MzYgMS45ODMgMS4zMDUuNDk2LjU3Ljc0NSAxLjI4Ny43NDUgMi4xNTR2MS4wMjFjMCAuNDcuMDUxLjg1NC4xNTMgMS4xNTIuMTAzLjI5OC4yNTYuNTI1LjQ2MS42ODIuMTkzLjE1Ny40MzcuMjYuNzMyLjMxMi4yOTUuMDUuNjIzLjA3Ni45ODQuMDc2aC45ODVabTE0LjMxNC03LjcwNmgtLjU4OGMtMS4xMDggMC0xLjg4OC4yMjMtMi4zNC42NjktLjQ1LjQ0NS0uNjc3IDEuMTc3LS42NzcgMi4xOTVWMTQuMWMwIDEuMTQ0LS4zNCAyLjAxMy0xLjAyIDIuNjA2LS42OC41OTMtMS42MDUuODktMi43NzQuODloLTIuMzg0di0xLjk4OGguOTg0Yy4zNjIgMCAuNjg4LS4wMjcuOTgtLjA4LjI5Mi0uMDU1LjUzOC0uMTU3LjczNy0uMzA4LjIwNC0uMTU3LjM1OC0uMzg0LjQ2LS42ODIuMTAzLS4yOTguMTU0LS42ODIuMTU0LTEuMTUydi0xLjAyYzAtLjg2OC4yNDgtMS41ODYuNzQ1LTIuMTU1LjQ5Ny0uNTcgMS4xNTgtMS4wMDQgMS45ODMtMS4zMDV2LS4yMTdjLS44MjUtLjMwMS0xLjQ4Ni0uNzM2LTEuOTgzLTEuMzA1LS40OTctLjU3LS43NDUtMS4yODgtLjc0NS0yLjE1NXYtMS4wMmMwLS40Ny0uMDUxLS44NTQtLjE1NC0xLjE1Mi0uMTAyLS4yOTgtLjI1Ni0uNTI2LS40Ni0uNjgyYTEuNzE5IDEuNzE5IDAgMCAwLS43MzctLjMwNyA1LjM5NSA1LjM5NSAwIDAgMC0uOTgtLjA4MmgtLjk4NFYwaDIuMzg0YzEuMTY5IDAgMi4wOTMuMjk3IDIuNzc0Ljg5LjY4LjU5MyAxLjAyIDEuNDYyIDEuMDIgMi42MDZ2MS4zNDZjMCAxLjAxOC4yMjYgMS43NS42NzggMi4xOTUuNDUxLjQ0NiAxLjIzMS42NjggMi4zNC42NjhoLjU4N3oiIGZpbGw9IiNmZmYiLz48L3N2Zz4=)](https://thanks.dev/u/gh/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
