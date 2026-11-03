//! # NEON backend
//!
//! The AVX-512 steps on 16-byte vectors, for aarch64.
//!
//! `tbl` compresses 8 lanes at a time from a shuffle table, and a mask sums
//! lane-weighted bits. Encoding spreads bytes with a three-table `tbl`, marks
//! unused lanes by their high bit and places the soft break with [`line()`].

use core::arch::aarch64::*;

use crate::{
    COMPRESS,
    codec::QuotedPrintableEol,
    encode::{ROOM, line},
    is_blank,
};

/// The lanes each of a group's three vectors takes from the three bytes each
/// input byte becomes, as indices into three 16-lane tables.
static SPREADS: [[u8; 16]; 3] = {
    let mut table = [[0; 16]; 3];
    let mut lane = 0;

    while lane < 48 {
        table[lane / 16][lane % 16] = (lane % 3 * 16 + lane / 3) as u8;
        lane += 1;
    }

    table
};

/// 0xFF at offset 15 alone: the 16 bytes from `16 - n` mark lane `n - 1`.
static LAST: [u8; 31] = {
    let mut table = [0; 31];
    table[15] = 0xff;
    table
};

/// Decodes like the portable blocks; `prev` tells whether a blank precedes
/// `src`.
#[target_feature(enable = "neon")]
pub(crate) fn decode(src: &[u8], dst: &mut [u8], prev: bool, dirty: &mut bool) -> [usize; 2] {
    let mut read = 0;
    let mut written = 0;
    let mut bad = 0;
    let [mut carry, mut prev] = [0, prev as u32];

    let digit = |v| {
        let letter = within(vorrq_u8(v, vdupq_n_u8(0x20)), b'a', b'f');
        let digit = vorrq_u8(within(v, b'0', b'9'), letter);
        let value = vandq_u8(v, vdupq_n_u8(0xf));
        (digit, vaddq_u8(value, vandq_u8(letter, vdupq_n_u8(9))))
    };

    while read < src.len() {
        let rest = src.len() - read;
        let [a, b, c] = vectors(&window::<18>(&src[read..]), 0);
        let is = |v, byte| vceqq_u8(v, vdupq_n_u8(byte));
        let [eq, cr, lf, tab] = b"=\r\n\t".map(|byte| mask(is(a, byte)));
        let blank = mask(is(a, b' ')) | tab;
        let lfb = mask(is(b, b'\n'));
        let [(high, high_value), (low, low_value)] = [b, c].map(digit);
        let escs = vandq_u8(vandq_u8(is(a, b'='), high), low);
        let esc = mask(escs);
        let soft = eq & lfb;
        let crlf = eq & mask(is(b, b'\r')) & mask(is(c, b'\n'));
        let spans = esc | crlf;
        let gone = (spans | soft) << 1 | spans << 2 | soft | crlf | carry;
        let slow = cr & !lfb | (blank << 1 | prev) & (cr | lf) | eq & !(spans | soft);
        let lower = mask(vcgeq_u8(a, vdupq_n_u8(0x60))) & gone;
        let text = mask(within(a, b' ', b'~'));
        let value = vorrq_u8(vshlq_n_u8::<4>(high_value), low_value);
        let out = vbslq_u8(escs, value, a);
        let valid = first(rest.min(16));
        let stop = (slow & valid).trailing_zeros() as usize;
        let done = valid & first(stop);
        bad |= (lower | !(text | tab | cr | lf)) & done;

        let mut spare = [0; 16];
        let direct = dst.len() - written >= 16;
        let to = match direct {
            true => dst[written..].as_mut_ptr(),
            false => spare.as_mut_ptr(),
        };
        // SAFETY: 16 bytes are writable from `to`.
        let n = unsafe { compress(out, done & !gone, to) };

        if !direct {
            dst[written..written + n].copy_from_slice(&spare[..n]);
        }

        written += n;

        if stop < 16 || rest <= 16 {
            read += stop.min(rest);
            break;
        }

        read += 16;
        [carry, prev] = [(spans | soft) >> 15 | spans >> 14, blank >> 15];
    }

    *dirty |= bad != 0;
    [read, written]
}

/// Encodes like the portable walk, in chunks of two groups of 16 bytes.
///
/// With `GUARD`, stops before a group where a `.`, `F` or `-` would start a
/// line, returning 1 as the third count.
#[target_feature(enable = "neon")]
pub(crate) fn encode<const TEXT: bool, const CRLF: bool, const GUARD: bool>(
    src: &[u8],
    dst: &mut [u8],
    last: bool,
    column: &mut usize,
) -> [usize; 3] {
    let eol = [QuotedPrintableEol::Lf, QuotedPrintableEol::Crlf][CRLF as usize];
    let (soft, soft_len) = eol.soft_break();
    let crlf = TEXT && CRLF;
    let base = dst.as_mut_ptr();
    let hex = load(b"0123456789ABCDEF");
    let spreads = SPREADS.map(|lanes| load(&lanes));
    let unused = vdupq_n_u8(0x80);
    let mut read = 0;
    let mut written = 0;
    let mut col = *column;

    while dst.len() - written >= ROOM {
        let rest = src.len() - read;
        let mut len = rest;

        // NOTE: unless `last`, a trailing blank or CR waits for the next byte.
        if !last {
            len -= (TEXT && src[read..].ends_with(b"\r")) as usize;
            len -= src[read..read + len].last().is_some_and(|&b| is_blank(b)) as usize;
        }

        // NOTE: 32 bytes at most, the encoder's carry, so `finish` stays small.
        let len = len.min(32);

        if len == 0 {
            break;
        }

        let bytes = window::<34>(&src[read..]);

        for half in 0..2 {
            let n = len.saturating_sub(16 * half).min(16);

            if n == 0 {
                break;
            }

            let [a, b, c] = vectors(&bytes, 16 * half);
            let end = match last && len == rest && 16 * half + n == len {
                true => load(LAST[16 - n..].first_chunk().unwrap()),
                false => vdupq_n_u8(0),
            };
            let is = |v, byte| vceqq_u8(v, vdupq_n_u8(byte));
            let blank = vorrq_u8(is(a, b' '), is(a, b'\t'));
            let plain = vorrq_u8(vbicq_u8(within(a, b' ', b'~'), is(a, b'=')), is(a, b'\t'));
            let mut esc = vorrq_u8(vmvnq_u8(plain), vandq_u8(blank, end));
            let [mut lf, mut drop] = [vdupq_n_u8(0); 2];

            if TEXT {
                let lf1 = is(b, b'\n');
                lf = is(a, b'\n');
                drop = vandq_u8(is(a, b'\r'), lf1);
                let breaks = vorrq_u8(lf1, vandq_u8(is(b, b'\r'), is(c, b'\n')));
                esc = vorrq_u8(vbicq_u8(esc, vorrq_u8(lf, drop)), vandq_u8(blank, breaks));
            }

            let nibbles = [vshrq_n_u8::<4>(a), vandq_u8(a, vdupq_n_u8(0xf))];
            let digits = nibbles.map(|v| vqtbl1q_u8(hex, v));
            let eol = vdupq_n_u8(if crlf { b'\r' } else { b'\n' });
            let eol = vbslq_u8(lf, eol, vbslq_u8(drop, unused, a));
            let second = if crlf {
                vbslq_u8(lf, vdupq_n_u8(b'\n'), unused)
            } else {
                unused
            };
            let takes = uint8x16x3_t(
                vbslq_u8(esc, vdupq_n_u8(b'='), eol),
                vbslq_u8(esc, digits[0], second),
                vbslq_u8(esc, digits[1], unused),
            );
            let lanes = spreads.map(|lanes| vqtbl3q_u8(takes, lanes));
            let keep = lanes
                .iter()
                .rev()
                .fold(0, |keep, &v| keep << 16 | mask(vcltq_u8(v, unused)) as u64);
            let keep = keep & ((1 << (3 * n)) - 1);
            let to = base.wrapping_add(written);
            let mut out = 0;

            for (i, v) in lanes.into_iter().enumerate() {
                // SAFETY: a chunk has `ROOM` bytes, more than its groups write.
                out += unsafe { compress(v, (keep >> (16 * i)) as u32, to.add(out)) };
            }

            let masks = [esc, lf, drop].map(|v| mask(v) & first(n));
            let marks = match GUARD {
                true => mask(vorrq_u8(vorrq_u8(is(a, b'.'), is(a, b'F')), is(a, b'-'))) & first(n),
                false => 0,
            };

            let Some((cut, next)) = line(masks, marks, out, col, crlf) else {
                *column = col;
                return [read + 16 * half, written, 1];
            };

            col = next;

            if let Some(cut) = cut {
                // SAFETY: as above.
                unsafe {
                    let tail = [0, 16, 32].map(|i| vld1q_u8(to.add(cut + i)));
                    to.add(cut).cast::<[u8; 4]>().write_unaligned(soft);

                    for (i, v) in tail.into_iter().enumerate() {
                        vst1q_u8(to.add(cut + soft_len + 16 * i), v);
                    }
                }

                out += soft_len;
            }

            written += out;
        }

        read += len;
    }

    *column = col;
    [read, written, 0]
}

/// The first `N` bytes of `src`, zeros past its end.
fn window<const N: usize>(src: &[u8]) -> [u8; N] {
    let mut bytes = [0; N];
    let n = src.len().min(N);
    bytes[..n].copy_from_slice(&src[..n]);
    bytes
}

/// The vectors at offsets `at`, `at + 1` and `at + 2` of `bytes`.
#[target_feature(enable = "neon")]
fn vectors<const N: usize>(bytes: &[u8; N], at: usize) -> [uint8x16_t; 3] {
    [0, 1, 2].map(|i| load(bytes[at + i..].first_chunk().unwrap()))
}

/// Writes the lanes of `v` that the low 16 bits of `keep` hold to `to`, in
/// order, and returns how many.
///
/// # Safety
///
/// 16 bytes are writable from `to`.
#[target_feature(enable = "neon")]
unsafe fn compress(v: uint8x16_t, keep: u32, to: *mut u8) -> usize {
    let mut n = 0;

    for (half, v) in [vget_low_u8(v), vget_high_u8(v)].into_iter().enumerate() {
        let lanes = (keep >> (8 * half) & 0xff) as usize;
        // SAFETY: an entry of `COMPRESS` holds the 8 bytes loaded.
        let shuffle = unsafe { vld1_u8(COMPRESS[lanes].as_ptr()) };
        // SAFETY: `n` is 8 at most, and 16 bytes are writable from `to`.
        unsafe { vst1_u8(to.add(n), vtbl1_u8(v, shuffle)) };
        n += lanes.count_ones() as usize;
    }

    n
}

/// The lanes of `v` from `low` to `high`.
#[target_feature(enable = "neon")]
fn within(v: uint8x16_t, low: u8, high: u8) -> uint8x16_t {
    vcleq_u8(vsubq_u8(v, vdupq_n_u8(low)), vdupq_n_u8(high - low))
}

/// A bit for each lane of the compare result `v`, summed half by half.
#[target_feature(enable = "neon")]
fn mask(v: uint8x16_t) -> u32 {
    let bits = vandq_u8(v, vreinterpretq_u8_u64(vdupq_n_u64(0x8040_2010_0804_0201)));
    vaddv_u8(vget_low_u8(bits)) as u32 | (vaddv_u8(vget_high_u8(bits)) as u32) << 8
}

/// The mask of the first `n` lanes, all 32 from `n` = 32 on.
fn first(n: usize) -> u32 {
    ((1_u64 << n.min(32)) - 1) as u32
}

#[target_feature(enable = "neon")]
fn load(bytes: &[u8; 16]) -> uint8x16_t {
    // SAFETY: `bytes` holds the 16 bytes loaded.
    unsafe { vld1q_u8(bytes.as_ptr()) }
}
