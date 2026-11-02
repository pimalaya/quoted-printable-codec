//! Throughput of quoted-printable-codec against encodify, the
//! quoted_printable crate and mail-parser, encoding and decoding three kinds
//! of body (ASCII text, accented text, binary), from 64 bytes to 10 MiB.
//! Throughput counts the raw bytes, on both sides.

use std::{hint::black_box, time::Duration};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use quoted_printable_codec::{
    codec::{QuotedPrintableCodec, QuotedPrintableEol, QuotedPrintableMode},
    cpu::QuotedPrintableCpu,
};
use simd_cpu::SimdCpu;

const SIZES: [usize; 4] = [64, 4 * 1024, 1024 * 1024, 10 * 1024 * 1024];

/// The codec of a body, with the best backend the CPU has.
fn codec(mode: QuotedPrintableMode) -> QuotedPrintableCodec {
    QuotedPrintableCodec {
        cpu: QuotedPrintableCpu::new(SimdCpu::detect()),
        eol: QuotedPrintableEol::Crlf,
        mode,
    }
}

/// A xorshift stream, the same every run.
fn random() -> impl Iterator<Item = u64> {
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;

    std::iter::repeat_with(move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    })
}

/// Words of 1 to 10 characters from `alphabet`, CRLF lines of about 72
/// characters, cut at `len` bytes.
fn prose(len: usize, alphabet: &[&str]) -> Vec<u8> {
    let mut out = Vec::with_capacity(len + 16);
    let mut column = 0;
    let mut random = random();

    while out.len() < len {
        let word = random.next().unwrap();

        for i in 0..(word % 10 + 1) {
            let pick = (word >> (8 + i * 5)) as usize % alphabet.len();
            out.extend_from_slice(alphabet[pick].as_bytes());
            column += 1;
        }

        if column > 72 {
            out.extend_from_slice(b"\r\n");
            column = 0;
        } else {
            out.push(if word >> 60 == 0 { b',' } else { b' ' });
            column += 1;
        }
    }

    out.truncate(len);
    out
}

const ASCII: [&str; 28] = [
    "e", "t", "a", "o", "i", "n", "s", "h", "r", "d", "l", "u", "c", "m", "w", "f", "g", "y", "p",
    "b", "v", "k", "e", "t", "a", "o", ".", "=",
];

const ACCENTED: [&str; 28] = [
    "e", "t", "a", "o", "i", "n", "s", "é", "r", "d", "l", "u", "c", "m", "è", "f", "g", "à", "p",
    "b", "v", "ç", "e", "t", "a", "o", ".", "ê",
];

/// The three bodies at `len` bytes, with the codec that encodes them.
fn inputs(len: usize) -> [(&'static str, Vec<u8>, QuotedPrintableCodec); 3] {
    let binary = random().take(len).map(|word| word as u8).collect();

    [
        (
            "ascii",
            prose(len, &ASCII),
            codec(QuotedPrintableMode::Text),
        ),
        (
            "accented",
            prose(len, &ACCENTED),
            codec(QuotedPrintableMode::Text),
        ),
        ("binary", binary, codec(QuotedPrintableMode::Binary)),
    ]
}

fn encodes(c: &mut Criterion) {
    for (kind, _, _) in inputs(0) {
        let mut group = c.benchmark_group(format!("encode {kind}"));
        group.warm_up_time(Duration::from_secs(1));
        group.measurement_time(Duration::from_secs(3));

        for size in SIZES {
            let (_, src, codec) = inputs(size).into_iter().find(|(k, ..)| *k == kind).unwrap();
            let mut out = vec![0; codec.encoded_len_max(size).unwrap()];
            group.throughput(Throughput::Bytes(size as u64));

            group.bench_with_input(
                BenchmarkId::new("quoted-printable-codec", size),
                &src,
                |b, src| b.iter(|| codec.encode(black_box(src), &mut out)),
            );

            let binary = codec.mode == QuotedPrintableMode::Binary;
            let engine = match binary {
                false => encodify::qp::BODY,
                true => encodify::qp::BINARY,
            };

            group.bench_with_input(BenchmarkId::new("encodify", size), &src, |b, src| {
                b.iter(|| engine.encode_slice(black_box(src), &mut out))
            });

            // NOTE: allocates its output, as its API asks.
            group.bench_with_input(
                BenchmarkId::new("quoted_printable", size),
                &src,
                |b, src| {
                    b.iter(|| match binary {
                        false => quoted_printable::encode(black_box(src)),
                        true => quoted_printable::encode_binary(black_box(src)),
                    })
                },
            );
        }

        group.finish();
    }
}

fn decodes(c: &mut Criterion) {
    for (kind, _, _) in inputs(0) {
        let mut group = c.benchmark_group(format!("decode {kind}"));
        group.warm_up_time(Duration::from_secs(1));
        group.measurement_time(Duration::from_secs(3));

        for size in SIZES {
            let (_, raw, codec) = inputs(size).into_iter().find(|(k, ..)| *k == kind).unwrap();
            let mut src = vec![0; codec.encoded_len_max(size).unwrap()];
            let len = codec.encode(&raw, &mut src).unwrap();
            src.truncate(len);

            let mut out = vec![0; codec.decoded_len_max(src.len()) + 64];
            group.throughput(Throughput::Bytes(size as u64));

            // NOTE: every decoder gives back the raw bytes, so they all do
            // the same work.
            let written = codec.decode(&src, &mut out).unwrap().written;
            assert_eq!(out[..written], raw);
            let written = encodify::qp::BODY.decode_slice(&src, &mut out).unwrap();
            assert_eq!(out[..written], raw);
            let decoded = quoted_printable::decode(&src, quoted_printable::ParseMode::Robust);
            assert_eq!(decoded.unwrap(), raw);
            let decoded = mail_parser::decoders::quoted_printable::quoted_printable_decode(&src);
            assert_eq!(decoded.unwrap(), raw);

            group.bench_with_input(
                BenchmarkId::new("quoted-printable-codec", size),
                &src,
                |b, src| b.iter(|| codec.decode(black_box(src), &mut out)),
            );

            group.bench_with_input(BenchmarkId::new("encodify", size), &src, |b, src| {
                b.iter(|| encodify::qp::BODY.decode_slice(black_box(src), &mut out))
            });

            // NOTE: the next two allocate their output, as their APIs ask.
            group.bench_with_input(
                BenchmarkId::new("quoted_printable", size),
                &src,
                |b, src| {
                    b.iter(|| {
                        quoted_printable::decode(
                            black_box(src),
                            quoted_printable::ParseMode::Robust,
                        )
                    })
                },
            );

            group.bench_with_input(BenchmarkId::new("mail-parser", size), &src, |b, src| {
                b.iter(|| {
                    mail_parser::decoders::quoted_printable::quoted_printable_decode(black_box(src))
                })
            });
        }

        group.finish();
    }
}

criterion_group!(benches, encodes, decodes);
criterion_main!(benches);
