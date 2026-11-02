# 🔣 quoted-printable-codec [![Matrix](https://img.shields.io/badge/chat-%23pimalaya-blue?style=flat&logo=matrix&logoColor=white)](https://matrix.to/#/#pimalaya:matrix.org) [![Mastodon](https://img.shields.io/badge/news-%40pimalaya-blue?style=flat&logo=mastodon&logoColor=white)](https://fosstodon.org/@pimalaya) [![Sponsor](https://img.shields.io/badge/sponsor-pink?style=flat&logo=github-sponsors&logoColor=white)](https://pimalaya.org/sponsor/)

Quoted-printable encoder and decoder with SIMD and no_std

## Table of contents

- [Features](#features)
- [RFC coverage](#rfc-coverage)
- [Usage](#usage)
- [Examples](#examples)
- [Testing](#testing)
- [Benchmarks](#benchmarks)
- [AI policy](https://github.com/pimalaya/.github/blob/master/AI_POLICY.md)
- [License](#license)
- [Social](#social)
- [Sponsoring](#sponsoring)

## Features

- **SIMD**: AVX-512 and AVX2 on x86_64, NEON on aarch64, the backend picked from the CPU token the application installs once, or one a caller passes; safe portable code anywhere else, and when nothing is installed on x86_64.
- **Fast**: up to 44x faster than encodify and 63x faster than the quoted_printable crate, see the [benchmarks](#benchmarks).
- **MIME lines**: encodes in lines of at most 76 characters, soft breaks included, ended by CRLF or LF.
- **Text and binary**: text mode keeps line breaks as hard breaks, binary mode escapes CR and LF like any other byte.
- **Header words**: Q mode writes and reads the RFC 2047 Q encoding of encoded words, with an exact `encoded_len` to size them.
- **Lenient decoding**: never fails, follows the robust rules of RFC 2045 (lowercase digits, trailing blanks, bare CRs, malformed escapes kept), and reports whether the input was clean.
- **I/O-free**: encode and decode chunk by chunk into your own buffers.
- **Portable**: no_std, the CPU features detected and installed once by the application through [simd-cpu](https://github.com/pimalaya/simd-cpu), the only dependency, and taken by every default codec; unsafe code only in the SIMD backends, which Miri and fuzz targets check against the portable one, vectorized by the compiler for any target.

## RFC coverage

| RFC    | What is covered                                                                                                        |
|--------|------------------------------------------------------------------------------------------------------------------------|
| [2045] | The quoted-printable content transfer encoding (6.7): escapes, soft line breaks, lines of 76 characters, robust decoding |
| [2047] | The Q encoding of encoded words (4.2), with the literal set a phrase allows (5(3)); splitting text into words is the caller's |

[2045]: https://www.rfc-editor.org/rfc/rfc2045#section-6.7
[2047]: https://www.rfc-editor.org/rfc/rfc2047#section-4.2

## Usage

The whole API is documented on [docs.rs](https://docs.rs/quoted-printable-codec/latest/quoted_printable_codec), with runnable snippets for the codec and its one-shot methods, the encoder and the decoder.

## Examples

The crate ships no examples folder, a whole program adding nothing to a two-call API. The crate root, the encoder and the decoder each open with a runnable snippet.

## Testing

The tests run every backend the machine has against the portable one. Miri and the fuzz targets need the nightly shell, fuzz/shell.nix; Miri reaches the AVX2 kernels only with their target features and interprets the NEON ones for aarch64 on any host, and the fuzz targets are described in [fuzz/README.md](./fuzz/README.md):

```sh
nix develop --command cargo test --release --all-features
nix-shell fuzz/shell.nix --run "cargo miri test --lib"
nix-shell fuzz/shell.nix --run "RUSTFLAGS='-C target-feature=+avx2,+bmi1,+lzcnt,+popcnt' cargo miri test --lib"
nix-shell fuzz/shell.nix --run "cargo miri test --lib --target aarch64-unknown-linux-gnu"
nix-shell fuzz/shell.nix --run "cargo fuzz run encode"
```

## Benchmarks

[criterion](https://crates.io/crates/criterion) medians of `cargo bench --bench compare` on 2026-11-01, on an AMD Ryzen AI Max+ 395 (Zen 5), in GiB/s of raw bytes. Each italic delta compares its row with quoted-printable-codec.

### ASCII decoding

| GiB/s | 64 B | 4 KiB | 1 MiB | 10 MiB |
| --- | ---: | ---: | ---: | ---: |
| **quoted-printable-codec** | **3.36** | **10.86** | **11.87** | **10.42** |
| [encodify](https://crates.io/crates/encodify) | *(-17%)* **2.79** | *(-76%)* **2.60** | *(-90%)* **1.20** | *(-89%)* **1.17** |
| [quoted_printable](https://crates.io/crates/quoted_printable) | *(-90%)* **0.33** | *(-95%)* **0.59** | *(-96%)* **0.49** | *(-95%)* **0.47** |
| [mail-parser](https://crates.io/crates/mail-parser) | *(-62%)* **1.26** | *(-88%)* **1.31** | *(-96%)* **0.53** | *(-95%)* **0.53** |

### Accented decoding

| GiB/s | 64 B | 4 KiB | 1 MiB | 10 MiB |
| --- | ---: | ---: | ---: | ---: |
| **quoted-printable-codec** | **3.37** | **7.45** | **7.62** | **7.20** |
| [encodify](https://crates.io/crates/encodify) | *(-68%)* **1.07** | *(-85%)* **1.14** | *(-94%)* **0.47** | *(-94%)* **0.47** |
| [quoted_printable](https://crates.io/crates/quoted_printable) | *(-94%)* **0.21** | *(-94%)* **0.41** | *(-96%)* **0.27** | *(-96%)* **0.27** |
| [mail-parser](https://crates.io/crates/mail-parser) | *(-72%)* **0.93** | *(-86%)* **1.01** | *(-95%)* **0.38** | *(-95%)* **0.37** |

### Binary decoding

| GiB/s | 64 B | 4 KiB | 1 MiB | 10 MiB |
| --- | ---: | ---: | ---: | ---: |
| **quoted-printable-codec** | **2.52** | **5.43** | **5.51** | **4.73** |
| [encodify](https://crates.io/crates/encodify) | *(-70%)* **0.75** | *(-86%)* **0.74** | *(-96%)* **0.24** | *(-95%)* **0.24** |
| [quoted_printable](https://crates.io/crates/quoted_printable) | *(-94%)* **0.15** | *(-95%)* **0.26** | *(-97%)* **0.15** | *(-97%)* **0.14** |
| [mail-parser](https://crates.io/crates/mail-parser) | *(-73%)* **0.68** | *(-86%)* **0.77** | *(-95%)* **0.25** | *(-95%)* **0.25** |

### ASCII encoding

| GiB/s | 64 B | 4 KiB | 1 MiB | 10 MiB |
| --- | ---: | ---: | ---: | ---: |
| **quoted-printable-codec** | **3.65** | **5.64** | **5.76** | **5.76** |
| [encodify](https://crates.io/crates/encodify) | *(-36%)* **2.34** | *(-59%)* **2.29** | *(-82%)* **1.04** | *(-82%)* **1.03** |
| [quoted_printable](https://crates.io/crates/quoted_printable) | *(-82%)* **0.65** | *(-87%)* **0.75** | *(-90%)* **0.58** | *(-90%)* **0.58** |

### Accented encoding

| GiB/s | 64 B | 4 KiB | 1 MiB | 10 MiB |
| --- | ---: | ---: | ---: | ---: |
| **quoted-printable-codec** | **2.16** | **5.72** | **5.77** | **5.75** |
| [encodify](https://crates.io/crates/encodify) | *(-54%)* **0.99** | *(-81%)* **1.09** | *(-93%)* **0.43** | *(-93%)* **0.42** |
| [quoted_printable](https://crates.io/crates/quoted_printable) | *(-82%)* **0.39** | *(-91%)* **0.49** | *(-95%)* **0.31** | *(-95%)* **0.31** |

### Binary encoding

| GiB/s | 64 B | 4 KiB | 1 MiB | 10 MiB |
| --- | ---: | ---: | ---: | ---: |
| **quoted-printable-codec** | **2.87** | **9.46** | **9.58** | **9.14** |
| [encodify](https://crates.io/crates/encodify) | *(-77%)* **0.65** | *(-93%)* **0.69** | *(-98%)* **0.22** | *(-98%)* **0.22** |
| [quoted_printable](https://crates.io/crates/quoted_printable) | *(-91%)* **0.25** | *(-97%)* **0.26** | *(-98%)* **0.16** | *(-98%)* **0.14** |

### Reading the numbers

- quoted-printable-codec runs its AVX-512 backend here, as `QuotedPrintableCpu::new(SimdCpu::detect())` picks it. A CPU without AVX-512 VBMI2 runs its AVX2 backend, and aarch64 its NEON one; neither is measured yet. The portable backend's cycle counts are in [benches/README.md](./benches/README.md).
- ASCII is lowercase words with a few `.`, `,` and `=` in CRLF lines of about 72 characters; accented is the same with about one character in seven a two-byte UTF-8 letter; binary is pseudo-random bytes, encoded in binary mode. Decoding reads quoted-printable-codec's encoding of each, and every decoder gives back the raw bytes.
- quoted_printable and mail-parser allocate their output, as their APIs ask; mail-parser only decodes.
- Each cell is the median of three runs: quoted-printable-codec's encoding cells moved by up to 4% between runs, its decoding cells by up to 11%.

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
