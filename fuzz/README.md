# Fuzzing

Two [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) targets, run from the repository root in the nightly shell:

```sh
nix-shell fuzz/shell.nix --run "cargo fuzz run decode"
nix-shell fuzz/shell.nix --run "cargo fuzz run encode"
```

- `decode`: every backend the machine has decodes any input, as a body or as Q text, in one call and resumed over chunks of any size into a buffer of any size, to the bytes and clean flag of the portable backend, without panicking.
- `encode`: the same for encoding, in text, binary or Q mode, with either line ending, `encoded_len` giving the exact length. The portable encoding must also decode back clean to the input, its LF and CRLF written as the codec's line ending in text mode; a body comes in lines of at most 76 characters with no CR outside a line ending, Q text with no line break at all.

The backends are the ones `simd_cpu::SimdCpu::detect` finds: on an x86_64 CPU with AVX-512 VBMI2, the AVX-512, AVX2 and portable ones.
