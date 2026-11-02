//! # AVX2 backend
//!
//! The AVX-512 steps on 32-byte vectors, for x86_64 CPUs without AVX-512
//! VBMI2. `vpshufb` compresses 8 lanes at a time from a shuffle table. No
//! step uses `pext` or `pdep`, microcoded on Zen 2. Encoding marks unused
//! lanes by their high bit and places the soft break with [`line()`].

use core::arch::x86_64::*;

use crate::{
    COMPRESS,
    codec::QuotedPrintableEol,
    encode::{ROOM, line},
    is_blank,
};

/// The lanes each of a group's three vectors takes from the three bytes each
/// input byte becomes; 0x80 elsewhere, zero to `vpshufb`.
static SPREADS: [[[u8; 16]; 3]; 3] = {
    let mut table = [[[0x80; 16]; 3]; 3];
    let mut lane = 0;

    while lane < 48 {
        table[lane / 16][lane % 3][lane % 16] = (lane / 3) as u8;
        lane += 1;
    }

    table
};

/// 0xFF at offset 31 alone: the 32 bytes from `32 - n` mark lane `n - 1`.
static LAST: [u8; 63] = {
    let mut table = [0; 63];
    table[31] = 0xff;
    table
};

/// Decodes like the portable blocks; `prev` tells whether a blank precedes
/// `src`.
#[target_feature(enable = "avx2,bmi1,lzcnt,popcnt")]
pub(crate) fn decode(src: &[u8], dst: &mut [u8], prev: bool, dirty: &mut bool) -> [usize; 2] {
    let mut read = 0;
    let mut written = 0;
    let mut bad = 0;
    let [mut carry, mut prev] = [0, prev as u32];

    let digit = |v| {
        let letter = within(_mm256_or_si256(v, splat(0x20)), b'a', b'f');
        let digit = _mm256_or_si256(within(v, b'0', b'9'), letter);
        let value = _mm256_and_si256(v, splat(0xf));
        (
            digit,
            _mm256_add_epi8(value, _mm256_and_si256(letter, splat(9))),
        )
    };

    while read < src.len() {
        let rest = src.len() - read;
        let [a, b, c] = window(&src[read..]);
        let is = |v, byte| mask(_mm256_cmpeq_epi8(v, splat(byte)));
        let [eq, cr, lf, tab] = b"=\r\n\t".map(|byte| is(a, byte));
        let blank = is(a, b' ') | tab;
        let lfb = is(b, b'\n');
        let [(high, high_value), (low, low_value)] = [b, c].map(digit);
        let escs = _mm256_and_si256(_mm256_cmpeq_epi8(a, splat(b'=')), high);
        let escs = _mm256_and_si256(escs, low);
        let esc = mask(escs);
        let soft = eq & lfb;
        let crlf = eq & is(b, b'\r') & is(c, b'\n');
        let spans = esc | crlf;
        let gone = (spans | soft) << 1 | spans << 2 | soft | crlf | carry;
        let slow = cr & !lfb | (blank << 1 | prev) & (cr | lf) | eq & !(spans | soft);
        let lower = mask(_mm256_cmpeq_epi8(_mm256_max_epu8(a, splat(0x60)), a)) & gone;
        let text = mask(within(a, b' ', b'~'));
        let high_value = _mm256_and_si256(_mm256_slli_epi16(high_value, 4), splat(0xf0));
        let value = _mm256_or_si256(high_value, low_value);
        let out = _mm256_blendv_epi8(a, value, escs);
        let valid = first(rest.min(32));
        let stop = (slow & valid).trailing_zeros() as usize;
        let done = valid & first(stop);
        bad |= (lower | !(text | tab | cr | lf)) & done;

        let keep = done & !gone;
        let mut spare = [0; 32];
        let direct = dst.len() - written >= 32;
        let to = match direct {
            true => dst[written..].as_mut_ptr(),
            false => spare.as_mut_ptr(),
        };
        // SAFETY: 32 bytes are writable from `to`; a half writes 16 at most.
        let n = unsafe { compress(_mm256_castsi256_si128(out), keep, to) };
        // SAFETY: as above.
        let n = n + unsafe { compress(_mm256_extracti128_si256(out, 1), keep >> 16, to.add(n)) };

        if !direct {
            dst[written..written + n].copy_from_slice(&spare[..n]);
        }

        written += n;

        if stop < 32 || rest <= 32 {
            read += stop.min(rest);
            break;
        }

        read += 32;
        [carry, prev] = [(spans | soft) >> 31 | spans >> 30, blank >> 31];
    }

    *dirty |= bad != 0;
    [read, written]
}

/// Encodes like the portable walk, in chunks of two groups of 16 bytes.
///
/// With `GUARD`, stops before a group where a `.`, `F` or `-` would start a
/// line, returning 1 as the third count.
#[target_feature(enable = "avx2,bmi1,lzcnt,popcnt")]
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
    let hex = _mm256_broadcastsi128_si256(load(b"0123456789ABCDEF"));
    let spreads =
        SPREADS.map(|vector| vector.map(|lanes| _mm256_broadcastsi128_si256(load(&lanes))));
    let mut read = 0;
    let mut written = 0;
    let mut col = *column;

    let bytes = |a, b, c, end| {
        let or = |a, b| _mm256_or_si256(a, b);
        let and = |a, b| _mm256_and_si256(a, b);
        let is = |v, byte| _mm256_cmpeq_epi8(v, splat(byte));
        let blank = or(is(a, b' '), is(a, b'\t'));
        let plain = or(
            _mm256_andnot_si256(is(a, b'='), within(a, b' ', b'~')),
            is(a, b'\t'),
        );
        let mut esc = or(_mm256_xor_si256(plain, splat(0xff)), and(blank, end));
        let [mut lf, mut drop] = [_mm256_setzero_si256(); 2];

        if TEXT {
            let lf1 = is(b, b'\n');
            lf = is(a, b'\n');
            drop = and(is(a, b'\r'), lf1);
            let breaks = or(lf1, and(is(b, b'\r'), is(c, b'\n')));
            esc = or(_mm256_andnot_si256(or(lf, drop), esc), and(blank, breaks));
        }

        let pick = |mask, yes, no| _mm256_blendv_epi8(no, yes, mask);
        let digits =
            [_mm256_srli_epi16(a, 4), a].map(|v| _mm256_shuffle_epi8(hex, and(v, splat(0xf))));
        let unused = splat(0x80);
        let eol = pick(
            lf,
            splat(if crlf { b'\r' } else { b'\n' }),
            pick(drop, unused, a),
        );
        let second = if crlf {
            pick(lf, splat(b'\n'), unused)
        } else {
            unused
        };
        let lanes = [
            pick(esc, splat(b'='), eol),
            pick(esc, digits[0], second),
            pick(esc, digits[1], unused),
        ];

        let marks = match GUARD {
            true => mask(or(or(is(a, b'.'), is(a, b'F')), is(a, b'-'))),
            false => 0,
        };
        let [esc, lf, drop] = [esc, lf, drop].map(|v| mask(v));

        (lanes, [esc, lf, drop, marks])
    };

    let mut group = |lanes: [__m128i; 3], [esc, lf, drop, marks]: [u32; 4], n: usize, at| {
        let to = base.wrapping_add(at);
        let keep = lanes.iter().rev().fold(0, |keep, &v| {
            keep << 16 | !_mm_movemask_epi8(v) as u64 & 0xffff
        });
        let keep = keep & ((1 << (3 * n)) - 1);
        let mut len = 0;

        for (i, v) in lanes.into_iter().enumerate() {
            // SAFETY: a chunk has `ROOM` bytes, more than its groups write.
            len += unsafe { compress(v, (keep >> (16 * i)) as u32, to.add(len)) };
        }

        let (cut, next) = line([esc, lf, drop], marks, len, col, crlf)?;
        col = next;

        let Some(cut) = cut else {
            return Some(len);
        };

        // SAFETY: as above.
        unsafe {
            let tail = [0, 16, 32].map(|i| _mm_loadu_si128(to.add(cut + i).cast()));
            to.add(cut).cast::<[u8; 4]>().write_unaligned(soft);

            for (i, v) in tail.into_iter().enumerate() {
                _mm_storeu_si128(to.add(cut + soft_len + 16 * i).cast(), v);
            }
        }

        Some(len + soft_len)
    };

    while dst.len() - written >= ROOM {
        let rest = src.len() - read;
        let mut len = rest;

        // NOTE: unless `last`, a trailing blank or CR waits for the next byte.
        if !last {
            len -= (TEXT && src[read..].ends_with(b"\r")) as usize;
            len -= src[read..read + len].last().is_some_and(|&b| is_blank(b)) as usize;
        }

        let len = len.min(32);

        if len == 0 {
            break;
        }

        let [a, b, c] = window(&src[read..]);
        let end = match last && len == rest {
            // SAFETY: `len` is 1 to 32, so `LAST` holds 32 bytes from there.
            true => unsafe { _mm256_loadu_si256(LAST.as_ptr().add(32 - len).cast()) },
            false => _mm256_setzero_si256(),
        };
        let (lanes, masks) = bytes(a, b, c, end);
        let spread = spreads.map(|[one, two, three]| {
            let take = |v, index| _mm256_shuffle_epi8(v, index);
            let v = _mm256_or_si256(take(lanes[0], one), take(lanes[1], two));
            _mm256_or_si256(v, take(lanes[2], three))
        });
        let halves = [
            spread.map(|v| _mm256_castsi256_si128(v)),
            spread.map(|v| _mm256_extracti128_si256(v, 1)),
        ];

        for (i, lanes) in halves.into_iter().enumerate() {
            let n = len.saturating_sub(16 * i).min(16);

            if n == 0 {
                break;
            }

            let masks = masks.map(|mask| mask >> (16 * i) & first(n));

            let Some(len) = group(lanes, masks, n, written) else {
                *column = col;
                return [read + 16 * i, written, 1];
            };

            written += len;
        }

        read += len;
    }

    *column = col;
    [read, written, 0]
}

/// The vectors at offsets 0, 1 and 2 of `src`, zeros past its end.
#[target_feature(enable = "avx2")]
fn window(src: &[u8]) -> [__m256i; 3] {
    let mut pad = [0; 34];
    let bytes = match src.first_chunk::<34>() {
        Some(bytes) => bytes,
        None => {
            pad[..src.len()].copy_from_slice(src);
            &pad
        }
    };

    // SAFETY: `bytes` holds the 34 bytes loaded.
    [0, 1, 2].map(|i| unsafe { _mm256_loadu_si256(bytes.as_ptr().add(i).cast()) })
}

/// Writes the lanes of `v` that the low 16 bits of `keep` hold to `to`, in
/// order, and returns how many.
///
/// # Safety
///
/// 16 bytes are writable from `to`.
#[target_feature(enable = "avx2,popcnt")]
unsafe fn compress(v: __m128i, keep: u32, to: *mut u8) -> usize {
    let mut n = 0;

    for half in 0..2 {
        let lanes = (keep >> (8 * half) & 0xff) as usize;
        // SAFETY: an entry of `COMPRESS` holds the 8 bytes loaded.
        let shuffle = unsafe { _mm_loadl_epi64(COMPRESS[lanes].as_ptr().cast()) };
        let shuffle = _mm_add_epi8(shuffle, _mm_set1_epi8(8 * half as i8));
        // SAFETY: `n` is 8 at most, and 16 bytes are writable from `to`.
        unsafe { _mm_storel_epi64(to.add(n).cast(), _mm_shuffle_epi8(v, shuffle)) };
        n += lanes.count_ones() as usize;
    }

    n
}

/// The lanes of `v` from `low` to `high`.
#[target_feature(enable = "avx2")]
fn within(v: __m256i, low: u8, high: u8) -> __m256i {
    let offset = _mm256_sub_epi8(v, splat(low));
    _mm256_cmpeq_epi8(_mm256_min_epu8(offset, splat(high - low)), offset)
}

/// A bit for each lane of `v` with its high bit set.
#[target_feature(enable = "avx2")]
fn mask(v: __m256i) -> u32 {
    _mm256_movemask_epi8(v) as u32
}

/// The mask of the first `n` lanes, all 32 from `n` = 32 on.
fn first(n: usize) -> u32 {
    ((1_u64 << n.min(32)) - 1) as u32
}

#[target_feature(enable = "avx2")]
fn splat(byte: u8) -> __m256i {
    _mm256_set1_epi8(byte as i8)
}

#[target_feature(enable = "avx2")]
fn load(bytes: &[u8; 16]) -> __m128i {
    // SAFETY: `bytes` holds the 16 bytes loaded.
    unsafe { _mm_loadu_si128(bytes.as_ptr().cast()) }
}
