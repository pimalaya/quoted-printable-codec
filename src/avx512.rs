//! # AVX-512 backend
//!
//! Branch-free 64-byte blocks with the VBMI2 byte compress, the next block
//! read ahead. Decoding reads a block's escapes at once from loads at offsets
//! 0, 1 and 2, and compresses digits and soft breaks away. Encoding spreads
//! each group of 21 bytes over three lanes a byte, compresses unused lanes
//! away, and places the soft break from masks moved by `pext`.

use core::{arch::x86_64::*, hint::select_unpredictable};

use crate::{
    codec::QuotedPrintableEol,
    encode::{LINE, ROOM},
    is_blank,
};

/// For decoding, by low 6 bits: each hex digit (a mismatch elsewhere), its
/// value shifted high and its value. For encoding: the byte each lane of the
/// three groups takes, and the hex digit of each nibble.
static TABLES: [[u8; 64]; 7] = {
    let mut table = [[0; 64]; 7];
    let mut i = 0;

    while i < 64 {
        let [byte, value] = match i as u8 {
            low @ 0x30..=0x39 => [low, low - 0x30],
            low @ (0x01..=0x06 | 0x21..=0x26) => [low | 0x40, (low & 7) + 9],
            low => [low ^ 1, 0],
        };
        let spread = (i / 3) as u8;
        [table[0][i], table[1][i], table[2][i]] = [byte, value << 4, value];
        [table[3][i], table[4][i], table[5][i]] = [spread, spread + 21, spread + 42];
        table[6][i] = b"0123456789ABCDEF"[i % 16];
        i += 1;
    }

    table
};

/// The first of the three lanes each byte of a group spreads over.
const STARTS: u64 = 0x1249_2492_4924_9249;

/// Decodes like the portable blocks; `prev` tells whether a blank precedes
/// `src`.
#[target_feature(enable = "avx512vbmi2,avx512vbmi,bmi1,bmi2,lzcnt,popcnt")]
pub(crate) fn decode(src: &[u8], dst: &mut [u8], prev: bool, dirty: &mut bool) -> [usize; 2] {
    let [check, high, low, ..] = TABLES.map(|table| load(&table));
    let mut read = 0;
    let mut written = 0;
    let mut bad = 0;

    let scan = |read: usize, carry: u64, prev: u64| {
        let rest = src.len() - read;
        let at = src.as_ptr().wrapping_add(read);
        // SAFETY: 66 bytes are left from `at`, or the masks cover the rest.
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
        let gone = (spans | soft) << 1 | spans << 2 | soft | crlf | carry;
        let slow = cr & !lfb | (blank << 1 | prev) & (cr | lf) | eq & !(spans | soft);
        let lower = _mm512_cmpge_epu8_mask(a, splat(0x60)) & gone;
        let value = _mm512_or_si512(
            _mm512_permutexvar_epi8(b, high),
            _mm512_permutexvar_epi8(c, low),
        );
        let out = _mm512_mask_blend_epi8(esc, a, value);
        let text = _mm512_cmplt_epu8_mask(_mm512_sub_epi8(a, splat(b' ')), splat(0x5f));
        let next = [(spans | soft) >> 63 | spans >> 62, blank >> 63];
        (out, [gone, slow, lower | !(text | tab | cr | lf)], next)
    };

    // NOTE: the next block is scanned ahead, so that blocks overlap.
    let mut next = (read < src.len()).then(|| scan(read, 0, prev as u64));

    while let Some((out, [gone, slow, flags], [carry, prev])) = next {
        let rest = src.len() - read;
        next = (rest > 64).then(|| scan(read + 64, carry, prev));
        let valid = first(rest);
        let stop = (slow & valid).trailing_zeros() as usize;
        let done = _bzhi_u64(valid, stop as u32);
        let keep = done & !gone;
        bad |= flags & done;
        let out = _mm512_maskz_compress_epi8(keep, out);
        let to = dst.as_mut_ptr().wrapping_add(written);
        // SAFETY: `dst` has at least as many bytes left as `src`.
        unsafe { _mm512_mask_storeu_epi8(to.cast(), valid, out) };
        written += keep.count_ones() as usize;

        if stop < 64 || rest <= 64 {
            read += stop.min(rest);
            break;
        }

        read += 64;
    }

    *dirty |= bad != 0;
    [read, written]
}

/// Encodes like the portable walk, in chunks of three groups of 21 bytes.
#[target_feature(enable = "avx512vbmi2,avx512vbmi,bmi1,bmi2,lzcnt,popcnt")]
pub(crate) fn encode<const TEXT: bool, const CRLF: bool>(
    src: &[u8],
    dst: &mut [u8],
    last: bool,
    column: &mut usize,
) -> [usize; 3] {
    let eol = [QuotedPrintableEol::Lf, QuotedPrintableEol::Crlf][CRLF as usize];
    let (soft, soft_len) = eol.soft_break();
    let crlf = TEXT && CRLF;
    let base = dst.as_mut_ptr();
    // NOTE: opaque, or LLVM fuses the spreads into `vpermt2b`, measured slower.
    let [_, _, _, spreads @ .., hex] = core::hint::black_box(&TABLES).each_ref().map(|t| load(t));
    let mut read = 0;
    let mut written = 0;
    let mut col = *column;

    let bytes = |a, b, end: u64| {
        let is = |v, byte| _mm512_cmpeq_epi8_mask(v, splat(byte));
        let blank = is(a, b' ') | is(a, b'\t');
        let [mut esc, mut lf, mut drop] = [!plain(a) | blank & end, 0, 0];

        if TEXT {
            let lf1 = is(b, b'\n');
            lf = is(a, b'\n');
            drop = is(a, b'\r') & lf1;
            esc = esc & !lf & !drop | blank & (lf1 | is(b, b'\r') & lf1 >> 1);
        }

        let set = |v, mask, byte| _mm512_mask_mov_epi8(v, mask, splat(byte));
        let high = _mm512_permutexvar_epi8(_mm512_srli_epi16(a, 4), hex);
        let lead = set(set(a, esc, b'='), lf, if crlf { b'\r' } else { b'\n' });
        let low = _mm512_permutexvar_epi8(a, hex);
        ([lead, set(high, lf, b'\n'), low], [esc, lf, drop])
    };

    let mut group = |[lead, high, low]: [__m512i; 3], spread, masks: [u64; 3], n: usize, at| {
        let [escs, lfs, drops] = masks.map(|mask| _pdep_u64(mask, STARTS));
        let starts = _bzhi_u64(STARTS, 3 * n as u32) & !drops;
        let keep = starts | (escs | if crlf { lfs } else { 0 }) << 1 | escs << 2;
        let out = _mm512_permutexvar_epi8(spread, lead);
        let out = _mm512_mask_permutexvar_epi8(out, STARTS << 1, spread, high);
        let out = _mm512_mask_permutexvar_epi8(out, STARTS << 2, spread, low);
        let out = _mm512_maskz_compress_epi8(keep, out);
        let len = keep.count_ones() as usize;

        // NOTE: a soft break goes before the first unit past the line's room.
        let [starts, lfs] = [starts, lfs].map(|mask| _pext_u64(mask, keep));
        let room = LINE - col;
        let upto = _bzhi_u64(!0, room as u32 + 1);
        let lz = _lzcnt_u64(starts & upto) as usize;
        let need = (room < len) & (lfs & upto == 0);
        let cut = select_unpredictable(need, 63 ^ lz, 64);
        let to = base.wrapping_add(at);

        // SAFETY: 68 bytes are left from `to`, as a chunk has `ROOM`.
        unsafe {
            _mm512_storeu_si512(to.cast(), out);
            to.add(cut).cast::<[u8; 4]>().write_unaligned(soft);
            _mm512_mask_storeu_epi8(to.add(soft_len).cast(), !_bzhi_u64(!0, cut as u32), out);
        }

        let wrap = select_unpredictable(need, (len + lz).wrapping_sub(63), col + len);
        let after = (len + _lzcnt_u64(lfs) as usize).wrapping_sub(64 + crlf as usize);
        col = select_unpredictable(lfs == 0, wrap, after);
        len + select_unpredictable(need, soft_len, 0)
    };

    // NOTE: the next chunk is read ahead, so that chunks overlap.
    let full = |read: usize| src.len() - read >= 65 - last as usize;
    let chunk = |read: usize| {
        let at = src.as_ptr().wrapping_add(read);
        // SAFETY: 64 bytes are left from `read`, and the mask covers the rest.
        let [a, b] = unsafe {
            let b = _mm512_maskz_loadu_epi8(first(src.len() - read - 1), at.add(1).cast());
            [_mm512_loadu_si512(at.cast()), b]
        };
        bytes(a, b, 0)
    };
    let mut next = full(read).then(|| chunk(read));

    while let Some((bytes, masks)) = next.filter(|_| dst.len() - written >= ROOM) {
        next = full(read + 63).then(|| chunk(read + 63));

        for (i, spread) in spreads.into_iter().enumerate() {
            let masks = masks.map(|mask| mask >> (21 * i));
            written += group(bytes, spread, masks, 21, written);
        }

        read += 63;
    }

    // NOTE: unless `last`, a trailing blank or CR waits for the next byte.
    let rest = src.len() - read;
    let mut len = rest;

    if !last {
        len -= (TEXT && src[read..].ends_with(b"\r")) as usize;
        len -= src[read..read + len].last().is_some_and(|&b| is_blank(b)) as usize;
    }

    let len = len.min(63) * (dst.len() - written >= ROOM) as usize;
    let at = src.as_ptr().wrapping_add(read);
    // SAFETY: the masks cover the bytes left from `read` only.
    let [a, b] = [0, 1].map(|i| unsafe {
        _mm512_maskz_loadu_epi8(first(rest.saturating_sub(i)), at.wrapping_add(i).cast())
    });
    let end = (first(rest) ^ first(rest.saturating_sub(1))) & (last as u64).wrapping_neg();
    let (bytes, masks) = bytes(a, b, end);

    for (i, spread) in spreads.into_iter().enumerate() {
        let n = len.saturating_sub(21 * i).min(21);

        if n == 0 {
            break;
        }

        let masks = masks.map(|mask| mask >> (21 * i) & first(n));
        written += group(bytes, spread, masks, n, written);
        read += n;
    }

    *column = col;
    [read, written, 0]
}

/// Whether `src` is long or has more than 4 bytes to escape: below that, the
/// portable walk wins.
#[target_feature(enable = "avx512bw,bmi2,popcnt")]
pub(crate) fn dense(src: &[u8]) -> bool {
    // SAFETY: the mask covers `src` only.
    let a = unsafe { _mm512_maskz_loadu_epi8(first(src.len()), src.as_ptr().cast()) };
    src.len() > 64 || (!plain(a) & first(src.len())).count_ones() > 4
}

/// The lanes of `a` copied as they are, as [`crate::is_plain`] tells.
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
