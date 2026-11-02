//! Every backend encodes any input in any mode and line ending, one-shot or
//! resumed over any chunks and room, to the bytes of the portable one, as
//! many as `encoded_len` tells, which decode back clean to the input, its
//! line breaks as the mode writes them: a body in lines of at most 76
//! characters, none starting with `.`, `F` or `-` when guarded, Q text with
//! no line break at all.

#![no_main]

use libfuzzer_sys::fuzz_target;
use quoted_printable_codec::{
    codec::{QuotedPrintableCodec, QuotedPrintableEol, QuotedPrintableMode},
    cpu::QuotedPrintableCpu,
    encode::QuotedPrintableEncode,
};
use simd_cpu::{SimdCpu, SimdFeature};

fuzz_target!(|data: &[u8]| {
    let [options, chunk, room, input @ ..] = data else {
        return;
    };

    let eol = match options % 2 {
        0 => QuotedPrintableEol::Crlf,
        _ => QuotedPrintableEol::Lf,
    };

    let mode = match options / 2 % 3 {
        0 => QuotedPrintableMode::Text,
        1 => QuotedPrintableMode::Binary,
        _ => QuotedPrintableMode::Q,
    };

    let portable = QuotedPrintableCodec {
        cpu: QuotedPrintableCpu::new(SimdCpu::NONE),
        eol,
        mode,
        guard: options / 6 % 2 == 1,
    };

    let expected = one_shot(input, portable);
    assert_eq!(portable.encoded_len(input), expected.len());
    let ending: &[u8] = match eol {
        QuotedPrintableEol::Crlf => b"\r\n",
        QuotedPrintableEol::Lf => b"\n",
    };

    if mode == QuotedPrintableMode::Q {
        assert!(!expected.iter().any(|byte| matches!(byte, b'\r' | b'\n')));
    }

    for line in expected.split(|&byte| byte == b'\n') {
        let line = line
            .strip_suffix(&ending[..ending.len() - 1])
            .unwrap_or(line);
        assert!(
            mode == QuotedPrintableMode::Q || line.len() <= 76,
            "{line:?}"
        );
        assert!(!line.contains(&b'\r'), "{line:?}");

        if portable.guard && mode != QuotedPrintableMode::Q {
            assert!(!line.first().is_some_and(|byte| b".F-".contains(byte)));
        }
    }

    let mut decoded = vec![0; portable.decoded_len_max(expected.len())];
    let output = portable.decode(&expected, &mut decoded).unwrap();
    assert!(output.clean);
    assert_eq!(
        decoded[..output.written],
        breaks(input, ending, mode == QuotedPrintableMode::Text)
    );

    for cpu in cpus() {
        let codec = QuotedPrintableCodec { cpu, ..portable };
        assert_eq!(one_shot(input, codec), expected, "{codec:?}");

        let mut encoder = QuotedPrintableEncode::new(codec);
        let mut out = Vec::new();
        let mut buffer = vec![0; *room as usize + 1];

        for mut src in input.chunks(*chunk as usize + 1) {
            while !src.is_empty() {
                let output = encoder.resume(src, &mut buffer);
                out.extend_from_slice(&buffer[..output.written]);
                src = &src[output.read..];
            }
        }

        let mut tail = vec![0; 300];
        let end = encoder.finish(&mut tail).unwrap();
        out.extend_from_slice(&tail[..end]);
        assert_eq!(out, expected, "{codec:?} resumed");
    }
});

fn one_shot(input: &[u8], codec: QuotedPrintableCodec) -> Vec<u8> {
    let mut out = vec![0; codec.encoded_len_max(input.len()).unwrap()];
    let written = codec.encode(input, &mut out).unwrap();
    out.truncate(written);
    out
}

/// `input` as its encoding decodes back: with every LF and CRLF written
/// `ending` in text mode, as it is in the others.
fn breaks(input: &[u8], ending: &[u8], text: bool) -> Vec<u8> {
    let mut out = Vec::new();
    let mut rest = input;

    while let [byte, after @ ..] = rest {
        match (text, byte, after) {
            (true, b'\r', [b'\n', ..]) => {}
            (true, b'\n', _) => out.extend_from_slice(ending),
            _ => out.push(*byte),
        }

        rest = after;
    }

    out
}

/// The backends the machine has, portable first.
fn cpus() -> Vec<QuotedPrintableCpu> {
    let cpu = SimdCpu::detect();
    let narrow = cpu.without(SimdFeature::Avx512Vbmi2);
    let mut cpus = Vec::new();

    for cpu in [SimdCpu::NONE, narrow, cpu].map(QuotedPrintableCpu::new) {
        if !cpus.contains(&cpu) {
            cpus.push(cpu);
        }
    }

    cpus
}
