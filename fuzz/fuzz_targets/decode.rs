//! # Decode fuzz target
//!
//! Every backend decodes any input as a body or as Q text, one-shot or
//! resumed over any chunks and room, to the bytes and clean flag of the
//! portable one, without panicking.

#![no_main]

use libfuzzer_sys::fuzz_target;
use quoted_printable_codec::{
    codec::{QuotedPrintableCodec, QuotedPrintableMode},
    cpu::QuotedPrintableCpu,
    decode::QuotedPrintableDecode,
};
use simd_cpu::{SimdCpu, SimdFeature};

fuzz_target!(|data: &[u8]| {
    let [options, chunk, room, input @ ..] = data else {
        return;
    };

    let mode = match options % 2 {
        0 => QuotedPrintableMode::Text,
        _ => QuotedPrintableMode::Q,
    };

    let portable = QuotedPrintableCodec {
        cpu: QuotedPrintableCpu::new(SimdCpu::NONE),
        mode,
        ..QuotedPrintableCodec::default()
    };

    let expected = one_shot(input, portable);

    for cpu in cpus() {
        let codec = QuotedPrintableCodec { cpu, ..portable };
        assert_eq!(one_shot(input, codec), expected, "{codec:?}");

        let mut decoder = QuotedPrintableDecode::new(codec);
        let mut out = Vec::new();
        let mut buffer = vec![0; *room as usize + 1];

        for mut src in input.chunks(*chunk as usize + 1) {
            while !src.is_empty() {
                let output = decoder.resume(src, &mut buffer);
                out.extend_from_slice(&buffer[..output.written]);
                src = &src[output.read..];
            }
        }

        let mut tail = vec![0; 300];
        let end = decoder.finish(&mut tail).unwrap();
        out.extend_from_slice(&tail[..end.written]);
        assert_eq!((out, end.clean), expected, "{codec:?} resumed");
    }
});

fn one_shot(input: &[u8], codec: QuotedPrintableCodec) -> (Vec<u8>, bool) {
    let mut out = vec![0; codec.decoded_len_max(input.len())];
    let output = codec.decode(input, &mut out).unwrap();
    out.truncate(output.written);
    (out, output.clean)
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
