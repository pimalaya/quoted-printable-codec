//! # AVX-512 backend
//!
//! Blocks of 64 bytes with the VBMI2 byte compress, no branch on the data.
//! Decoding reads every escape of a block at once, from the block loaded
//! again one and two bytes on, and compresses the digits and soft breaks
//! away. Encoding spreads each byte of a group of 21 over three lanes, its
//! escape's, compresses away the lanes it does not use, and finds the one
//! soft break a group may need from masks moved to the output by `pext`.

use core::arch::x86_64::*;

use crate::{
    codec::QuotedPrintableCodec,
    encode::{LINE, ROOM},
    is_blank,
};

/// For the low 6 bits of each hex digit: the digit, its value shifted high
/// and its value; elsewhere a byte with other low bits.
static DIGITS: [[u8; 64]; 3] = {
    let mut table = [[0; 64]; 3];
    let mut i = 0;

    while i < 64 {
        let [byte, value] = match i as u8 {
            low @ 0x30..=0x39 => [low, low - 0x30],
            low @ (0x01..=0x06 | 0x21..=0x26) => [low | 0x40, (low & 7) + 9],
            low => [low ^ 1, 0],
        };
        [table[0][i], table[1][i], table[2][i]] = [byte, value << 4, value];
        i += 1;
    }

    table
};

/// For each of the three groups of a chunk, the byte each lane takes, three
/// lanes a byte; then the hex digits.
static SPREAD: [[u8; 64]; 4] = {
    let mut table = [[0; 64]; 4];
    let mut i = 0;

    while i < 64 {
        let byte = (i / 3) as u8;
        [table[0][i], table[1][i], table[2][i]] = [byte, byte + 21, byte + 42];
        table[3][i] = b"0123456789ABCDEF"[i % 16];
        i += 1;
    }

    table
};

/// The first of the three lanes each byte of a group spreads over.
const STARTS: u64 = 0x1249_2492_4924_9249;

/// Decodes `src` into `dst`, at least as long, up to the first byte the
/// token steps decide or the end of `src`, as the portable blocks do;
/// `prev` tells whether a space or tab comes before `src`.
#[target_feature(enable = "avx512vbmi2,avx512vbmi,bmi1,bmi2,lzcnt,popcnt")]
pub(crate) fn decode(src: &[u8], dst: &mut [u8], prev: bool, dirty: &mut bool) -> [usize; 2] {
    let [check, high, low] = DIGITS.map(|table| load(&table));
    let mut read = 0;
    let mut written = 0;
    let mut carry = 0;
    let mut prev = prev as u64;
    let mut bad = 0;

    while read < src.len() {
        let rest = src.len() - read;
        let at = src.as_ptr().wrapping_add(read);
        // SAFETY: 66 bytes are left from `at`, or the masks cover the bytes
        // left only.
        let [a, b, c] = [0, 1, 2].map(|i| unsafe {
            match rest >= 66 {
                true => _mm512_loadu_si512(at.add(i).cast()),
                false => _mm512_maskz_loadu_epi8(
                    first(rest.saturating_sub(i)),
                    at.wrapping_add(i).cast(),
                ),
            }
        });
        let is = |v, byte| _mm512_cmpeq_epi8_mask(v, splat(byte));
        let [eq, cr, lf, tab] = b"=\r\n\t".map(|byte| is(a, byte));
        let blank = is(a, b' ') | tab;
        let lfb = is(b, b'\n');
        let esc = eq
            & _mm512_cmpeq_epi8_mask(_mm512_permutexvar_epi8(b, check), b)
            & _mm512_cmpeq_epi8_mask(_mm512_permutexvar_epi8(c, check), c);
        let soft = eq & lfb;
        let crlf = eq & is(b, b'\r') & is(c, b'\n');
        let spans = esc | crlf;
        // NOTE: the digits and soft breaks of this block, and the bytes of
        // this one the last block's run over.
        let gone = (spans | soft) << 1 | spans << 2 | soft | crlf | carry;
        let slow = cr & !lfb | (blank << 1 | prev) & (cr | lf) | eq & !(spans | soft);
        let valid = first(rest);
        let stop = (slow & valid).trailing_zeros() as usize;
        let done = _bzhi_u64(valid, stop as u32);
        let keep = done & !gone;
        let lower = _mm512_cmpge_epu8_mask(a, splat(0x60)) & gone;
        bad |= (lower | !(plain(a) | eq | cr | lf)) & done;

        let value = _mm512_or_si512(
            _mm512_permutexvar_epi8(b, high),
            _mm512_permutexvar_epi8(c, low),
        );
        let out = _mm512_maskz_compress_epi8(keep, _mm512_mask_blend_epi8(esc, a, value));
        let count = keep.count_ones() as usize;
        let to = dst.as_mut_ptr().wrapping_add(written);

        // SAFETY: `dst` has at least as many bytes left as `src`, so 64 when
        // the block is whole, else `count`.
        unsafe {
            match valid {
                u64::MAX => _mm512_storeu_si512(to.cast(), out),
                _ => _mm512_mask_storeu_epi8(to.cast(), first(count), out),
            }
        }

        written += count;

        if stop < 64 || rest <= 64 {
            read += stop.min(rest);
            break;
        }

        read += 64;
        carry = (spans | soft) >> 63 | spans >> 62;
        prev = blank >> 63;
    }

    *dirty |= bad != 0;
    [read, written]
}

/// Encodes `src` into `dst` from `column` in chunks of three groups of 21
/// bytes, as many as fit, as the portable walk does.
#[target_feature(enable = "avx512vbmi2,avx512vbmi,bmi1,bmi2,lzcnt,popcnt")]
pub(crate) fn encode<const TEXT: bool>(
    src: &[u8],
    dst: &mut [u8],
    last: bool,
    codec: QuotedPrintableCodec,
    column: &mut usize,
) -> [usize; 3] {
    let (soft, soft_len) = codec.eol.soft_break();
    let crlf = (TEXT && soft_len == 3) as u64;
    let [spreads @ .., hex] = SPREAD.map(|table| load(&table));
    let mut read = 0;
    let mut written = 0;
    let mut col = *column;

    // The bytes of `a` to escape, its line feeds and the CRs they drop;
    // `next` tells whether a line feed follows the last lane, `end` holds
    // the input's last byte.
    let masks = |a, next: bool, end: u64| {
        let is = |byte| _mm512_cmpeq_epi8_mask(a, splat(byte));
        let tab = is(b'\t');
        let blank = is(b' ') | tab;
        let esc = !plain(a) | blank & end;

        match TEXT {
            true => {
                let [cr, lf] = [is(b'\r'), is(b'\n')];
                let lf1 = lf >> 1 | (next as u64) << 63;
                let drop = cr & lf1;
                let brk = lf1 | cr >> 1 & (lf >> 2 | (next as u64) << 62);
                [esc & !lf & !drop | blank & brk, lf, drop]
            }
            false => [esc, 0, 0],
        }
    };

    // Encodes the `n` bytes of `a` the lanes of `spread` take, of the masks
    // `masks`, to `to`, returning the bytes written.
    let mut group = |a, spread, [esc, lf, drop]: [u64; 3], n: usize, to: *mut u8| {
        let starts = _bzhi_u64(STARTS, 3 * n as u32);
        let [escs, lfs, drops] = [esc, lf, drop].map(|mask| _pdep_u64(mask, STARTS));
        let crs = lfs & crlf.wrapping_neg();
        let keep = starts & !drops | (escs | crs) << 1 | escs << 2;

        let bytes = _mm512_permutexvar_epi8(spread, a);
        let nibbles = _mm512_mask_blend_epi8(
            STARTS << 1,
            _mm512_and_si512(bytes, splat(0x0f)),
            _mm512_and_si512(_mm512_srli_epi16(bytes, 4), splat(0x0f)),
        );
        let out = _mm512_mask_blend_epi8(STARTS, _mm512_permutexvar_epi8(nibbles, hex), bytes);
        let out = _mm512_mask_mov_epi8(out, escs, splat(b'='));
        let out = _mm512_mask_mov_epi8(out, crs, splat(b'\r'));
        let out = _mm512_mask_mov_epi8(out, crs << 1, splat(b'\n'));
        let out = _mm512_maskz_compress_epi8(keep, out);
        let len = keep.count_ones() as usize;

        // NOTE: a literal past the line's room, or an escape running past
        // it, takes a soft break before it, unless a line break comes first.
        let [lits, escs, lfs] =
            [starts & !drops & !escs & !lfs, escs, lfs].map(|mask| _pext_u64(mask, keep));
        let late = !_bzhi_u64(!0, (LINE - col) as u32);
        let cuts = (lits & late | escs & late >> 2) & _bzhi_u64(!0, lfs.trailing_zeros());
        let cut = cuts.trailing_zeros() as usize;
        let line = lfs << (1 + crlf) | cuts & cuts.wrapping_neg();

        // SAFETY: 68 bytes are left from `to`, as a chunk has `ROOM`.
        unsafe {
            _mm512_storeu_si512(to.cast(), out);
            to.add(cut).cast::<[u8; 4]>().write_unaligned(soft);
            _mm512_mask_storeu_epi8(to.add(soft_len).cast(), !first(cut), out);
        }

        col = match line {
            0 => col + len,
            _ => len + _lzcnt_u64(line) as usize - 63,
        };
        len + if cuts != 0 { soft_len } else { 0 }
    };

    while dst.len() - written >= ROOM && src.len() - read >= 65 - last as usize {
        // SAFETY: 64 bytes are left from `read`.
        let a = unsafe { _mm512_loadu_si512(src.as_ptr().add(read).cast()) };
        let masks = masks(a, src.get(read + 64) == Some(&b'\n'), 0);

        for (i, spread) in spreads.into_iter().enumerate() {
            let masks = masks.map(|mask| mask >> (21 * i) & 0x1f_ffff);
            written += group(a, spread, masks, 21, dst.as_mut_ptr().wrapping_add(written));
        }

        read += 63;
    }

    // NOTE: the input's end; unless `last`, a space, tab or CR ending `src`
    // waits for the next byte.
    let rest = src.len() - read;
    let mut len = rest;

    if !last {
        len -= (TEXT && src[read..].ends_with(b"\r")) as usize;
        len -= src[read..read + len].last().is_some_and(|&b| is_blank(b)) as usize;
    }

    let len = len.min(63) * (dst.len() - written >= ROOM) as usize;

    // SAFETY: the mask covers the bytes left from `read` only.
    let a = unsafe { _mm512_maskz_loadu_epi8(first(rest), src.as_ptr().add(read).cast()) };
    let end = match last && rest <= 64 {
        true => 1 << rest.saturating_sub(1),
        false => 0,
    };
    let masks = masks(a, src.get(read + 64) == Some(&b'\n'), end);

    for (i, spread) in spreads.into_iter().enumerate() {
        let n = len.saturating_sub(21 * i).min(21);

        if n == 0 {
            break;
        }

        let masks = masks.map(|mask| mask >> (21 * i) & first(n));
        written += group(a, spread, masks, n, dst.as_mut_ptr().wrapping_add(written));
        read += n;
    }

    *column = col;
    [read, written, 0]
}

/// Whether `src` is long or has more than a few bytes to escape: on a short
/// input with so few, the portable walk over them beats spreading them all.
#[target_feature(enable = "avx512bw,bmi2,popcnt")]
pub(crate) fn dense(src: &[u8]) -> bool {
    // SAFETY: the mask covers `src` only.
    let a = unsafe { _mm512_maskz_loadu_epi8(first(src.len()), src.as_ptr().cast()) };
    src.len() > 64 || (!plain(a) & first(src.len())).count_ones() > 4
}

/// The lanes of `a` both directions copy as they are, as [`crate::is_plain`]
/// tells.
#[target_feature(enable = "avx512bw")]
fn plain(a: __m512i) -> u64 {
    let is = |byte| _mm512_cmpeq_epi8_mask(a, splat(byte));
    _mm512_cmplt_epu8_mask(_mm512_sub_epi8(a, splat(b' ')), splat(0x5f)) & !is(b'=') | is(b'\t')
}

/// The mask of the first `n` lanes, all 64 from `n` = 64 on.
#[target_feature(enable = "bmi2")]
fn first(n: usize) -> u64 {
    _bzhi_u64(!0, n.min(64) as u32)
}

#[target_feature(enable = "avx512bw")]
fn splat(byte: u8) -> __m512i {
    _mm512_set1_epi8(byte as i8)
}

#[target_feature(enable = "avx512f")]
fn load(table: &[u8; 64]) -> __m512i {
    // SAFETY: `table` holds the 64 bytes loaded.
    unsafe { _mm512_loadu_si512(table.as_ptr().cast()) }
}
