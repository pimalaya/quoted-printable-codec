//! Throughput of quoted-printable-codec against encodify, the
//! quoted_printable crate and mail-parser, encoding and decoding three kinds
//! of body (ASCII text, accented text, binary), from 64 bytes to 1 MiB.
//! Throughput counts the raw bytes, on both sides.

use std::{hint::black_box, time::Duration};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use quoted_printable_codec::{
    decode::{decode, decoded_len_max},
    encode::{QuotedPrintableEncodeOptions, encode, encoded_len_max},
};

const SIZES: [usize; 3] = [64, 4 * 1024, 1024 * 1024];

const TEXT: QuotedPrintableEncodeOptions = QuotedPrintableEncodeOptions {
    eol: quoted_printable_codec::encode::QuotedPrintableEol::Crlf,
    binary: false,
};

const BINARY: QuotedPrintableEncodeOptions = QuotedPrintableEncodeOptions {
    eol: quoted_printable_codec::encode::QuotedPrintableEol::Crlf,
    binary: true,
};

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

/// The three bodies at `len` bytes, with the options that encode them.
fn inputs(len: usize) -> [(&'static str, Vec<u8>, QuotedPrintableEncodeOptions); 3] {
    let binary = random().take(len).map(|word| word as u8).collect();

    [
        ("ascii", prose(len, &ASCII), TEXT),
        ("accented", prose(len, &ACCENTED), TEXT),
        ("binary", binary, BINARY),
    ]
}

fn encodes(c: &mut Criterion) {
    for (kind, _, _) in inputs(0) {
        let mut group = c.benchmark_group(format!("encode {kind}"));
        group.warm_up_time(Duration::from_secs(1));
        group.measurement_time(Duration::from_secs(3));

        for size in SIZES {
            let (_, src, opts) = inputs(size).into_iter().find(|(k, ..)| *k == kind).unwrap();
            let mut out = vec![0; encoded_len_max(size, opts).unwrap()];
            group.throughput(Throughput::Bytes(size as u64));

            group.bench_with_input(
                BenchmarkId::new("quoted-printable-codec", size),
                &src,
                |b, src| b.iter(|| encode(black_box(src), &mut out, opts)),
            );

            let engine = match opts.binary {
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
                    b.iter(|| match opts.binary {
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
            let (_, raw, opts) = inputs(size).into_iter().find(|(k, ..)| *k == kind).unwrap();
            let mut src = vec![0; encoded_len_max(size, opts).unwrap()];
            let len = encode(&raw, &mut src, opts).unwrap();
            src.truncate(len);

            let mut out = vec![0; decoded_len_max(src.len()) + 64];
            group.throughput(Throughput::Bytes(size as u64));

            // NOTE: every decoder gives back the raw bytes, so they all do
            // the same work.
            let written = decode(&src, &mut out).unwrap().written;
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
                |b, src| b.iter(|| decode(black_box(src), &mut out)),
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
