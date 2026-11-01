//! # quoted-printable-codec
//!
//! Quoted-printable encoding and decoding of MIME bodies (RFC 2045 6.7),
//! lenient as mail needs it, `no_std`, with no dependency, no I/O and no
//! unsafe code.
//!
//! A [`QuotedPrintableCodec`], from [`codec`], holds what every call runs:
//! the mode, text or binary, and the line ending of encoded output. Its
//! one-shot methods take a whole input at once. [`QuotedPrintableEncode`] and
//! [`QuotedPrintableDecode`], from [`encode`] and [`decode`], take a codec,
//! then the caller's input and output buffers chunk by chunk, as any I/O-free
//! coroutine does.
//!
//! Both directions read their input in blocks of 32 bytes. A block takes a
//! bitmask of its special bytes, built 16 lanes at a time in a shape
//! compilers turn into vector compares on any target, and walks its set bits
//! only: escapes, soft and hard line breaks, each literal run between them
//! copied whole. The decoder leaves to token steps, byte by byte, what a
//! block does not decide alone: malformed escapes, bare CRs, blanks before a
//! line break and the end of the input. A block reads the last bytes of an
//! input from a padded copy of them. The coroutines run the blocks straight
//! between the caller's buffers, and keep only the few bytes they cannot
//! decide yet, or have no room for, between two calls.
//!
//! ```
//! use quoted_printable_codec::codec::{QuotedPrintableCodec, QuotedPrintableEol};
//!
//! let input = "Grüße,\nJürgen\n".as_bytes();
//! let codec = QuotedPrintableCodec {
//!     eol: QuotedPrintableEol::Crlf,
//!     binary: false,
//! };
//!
//! let mut encoded = [0; 256];
//! assert!(encoded.len() >= codec.encoded_len_max(input.len()).unwrap());
//! let len = codec.encode(input, &mut encoded).unwrap();
//! assert_eq!(&encoded[..len], b"Gr=C3=BC=C3=9Fe,\r\nJ=C3=BCrgen\r\n");
//!
//! let mut decoded = [0; 64];
//! let output = codec.decode(&encoded[..len], &mut decoded).unwrap();
//! assert_eq!(&decoded[..output.written], "Grüße,\r\nJürgen\r\n".as_bytes());
//! assert!(output.clean);
//! ```
//!
//! [`QuotedPrintableCodec`]: codec::QuotedPrintableCodec
//! [`QuotedPrintableEncode`]: encode::QuotedPrintableEncode
//! [`QuotedPrintableDecode`]: decode::QuotedPrintableDecode

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", test))]
extern crate alloc;

pub mod codec;
pub mod decode;
mod edge;
pub mod encode;

/// The bytes both directions copy as they are: printable ASCII but `=`,
/// space and tab.
const fn is_plain(byte: u8) -> bool {
    (byte.wrapping_sub(b' ') < 0x5f) & (byte != b'=') | (byte == b'\t')
}

const fn is_blank(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}

/// The lanes of `bytes` in `class`, a bit each, in a shape compilers turn
/// into vector compares.
#[inline(never)]
fn mask(bytes: &[u8; 64], class: impl Fn(u8) -> bool) -> u32 {
    let (halves, _) = bytes.as_chunks();
    half(&halves[0], &class) | half(&halves[1], &class) << 16
}

/// NOTE: compilers vectorize a fold of 16 lanes, not one of 32.
#[inline(always)]
fn half(bytes: &[u8; 16], class: &impl Fn(u8) -> bool) -> u32 {
    let lanes = bytes.iter().enumerate();
    lanes.fold(0, |mask, (i, &byte)| mask | (class(byte) as u32) << i)
}

/// Copies the last 64 bytes of `src`, or all of them when fewer, to end at
/// offset 64 of `out`, zeros after them, for blocks to read the end of an
/// input 64 bytes at a time.
fn pad(out: &mut [u8; 128], src: &[u8]) {
    match src.last_chunk::<64>() {
        Some(last) => out[..64].copy_from_slice(last),
        None => out[64 - src.len()..64].copy_from_slice(src),
    }
}

/// The 64 bytes a block at the start of `src` reads, from `end` as [`pad`]
/// fills it when fewer remain.
fn window<'a>(src: &'a [u8], end: &'a [u8; 128]) -> &'a [u8; 64] {
    src.first_chunk()
        .unwrap_or_else(|| end[64 - src.len()..].first_chunk().unwrap())
}

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};

    use crate::{
        codec::{QuotedPrintableCodec, QuotedPrintableEol},
        decode::*,
        encode::*,
    };

    /// A xorshift stream, the same every run.
    struct Random(u64);

    impl Random {
        fn below(&mut self, n: usize) -> usize {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0 as usize % n
        }
    }

    /// Bodies of up to 300 bytes, mostly the bytes both directions treat
    /// apart.
    fn bodies(random: &mut Random) -> Vec<Vec<u8>> {
        let bytes = b"aaaaaaaaaaaaaaaaaaaaaaaa==== \t\t\r\n\r\n09AFaf\x00\x7f\xc3\xff";

        (0..2000)
            .map(|_| {
                let len = random.below(300);
                (0..len).map(|_| bytes[random.below(bytes.len())]).collect()
            })
            .collect()
    }

    fn codecs() -> [QuotedPrintableCodec; 4] {
        [false, true]
            .map(|binary| {
                [QuotedPrintableEol::Crlf, QuotedPrintableEol::Lf]
                    .map(|eol| QuotedPrintableCodec { eol, binary })
            })
            .concat()
            .try_into()
            .unwrap()
    }

    fn encoded(src: &[u8], codec: QuotedPrintableCodec) -> Vec<u8> {
        let mut out = vec![0; codec.encoded_len_max(src.len()).unwrap()];
        let written = codec.encode(src, &mut out).unwrap();
        out.truncate(written);
        out
    }

    fn decoded(src: &[u8]) -> (Vec<u8>, bool) {
        let codec = QuotedPrintableCodec::default();
        let mut out = vec![0; codec.decoded_len_max(src.len())];
        let output = codec.decode(src, &mut out).unwrap();
        out.truncate(output.written);
        (out, output.clean)
    }

    #[test]
    fn every_split_gives_the_same_output() {
        let mut random = Random(0x1234_5678_9abc_def1);

        for body in bodies(&mut random) {
            for codec in codecs() {
                let mut encoder = QuotedPrintableEncode::new(codec);
                let mut out = Vec::new();

                for chunk in body.chunks(1 + random.below(40)) {
                    let mut chunk = chunk;

                    while !chunk.is_empty() {
                        let mut room = vec![0; 1 + random.below(200)];
                        let output = encoder.resume(chunk, &mut room);
                        out.extend_from_slice(&room[..output.written]);
                        chunk = &chunk[output.read..];
                    }
                }

                let mut room = vec![0; 300];
                let end = encoder.finish(&mut room).unwrap();
                out.extend_from_slice(&room[..end]);
                assert_eq!(out, encoded(&body, codec));
            }

            for src in [
                body.clone(),
                encoded(&body, QuotedPrintableCodec::default()),
            ] {
                let mut decoder = QuotedPrintableDecode::new(QuotedPrintableCodec::default());
                let mut out = Vec::new();

                for chunk in src.chunks(1 + random.below(40)) {
                    let mut chunk = chunk;

                    while !chunk.is_empty() {
                        let mut room = vec![0; 1 + random.below(100)];
                        let output = decoder.resume(chunk, &mut room);
                        out.extend_from_slice(&room[..output.written]);
                        chunk = &chunk[output.read..];
                    }
                }

                let mut room = vec![0; 300];
                let end = decoder.finish(&mut room).unwrap();
                out.extend_from_slice(&room[..end.written]);
                assert_eq!((out, end.clean), decoded(&src));
            }
        }
    }

    #[test]
    fn a_frozen_corpus_keeps_its_output() {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        let mut fold = |bytes: &[u8]| {
            for &byte in bytes {
                hash = (hash ^ byte as u64).wrapping_mul(0x100_0000_01b3);
            }
        };

        for body in bodies(&mut Random(0x0f1e_2d3c_4b5a_6978)) {
            for codec in codecs() {
                let out = encoded(&body, codec);
                let (back, clean) = decoded(&out);
                fold(&out);
                fold(&back);
                fold(&[clean as u8]);
            }

            let (out, clean) = decoded(&body);
            fold(&out);
            fold(&[clean as u8]);
        }

        assert_eq!(hash, 5_623_604_759_311_401_023);
    }

    #[test]
    fn a_finished_coroutine_starts_over() {
        let codec = QuotedPrintableCodec::default();
        let mut encoder = QuotedPrintableEncode::new(codec);
        let mut decoder = QuotedPrintableDecode::new(codec);
        let mut out = [0; 512];

        for src in [&b"a=zb"[..], &[b'x'; 50], &[b'x'; 50]] {
            let written = encoder.resume(src, &mut out).written;
            let end = encoder.finish(&mut out[written..]).unwrap();
            assert_eq!(out[..written + end], encoded(src, codec));
        }

        for src in [&b"a=zb"[..], b"a=3Db"] {
            let written = decoder.resume(src, &mut out).written;
            let end = decoder.finish(&mut out[written..]).unwrap();
            assert_eq!(
                (out[..written + end.written].to_vec(), end.clean),
                decoded(src)
            );
        }
    }
}
