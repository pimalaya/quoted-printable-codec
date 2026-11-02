//! # Call edges
//!
//! What a coroutine keeps between calls: undecided input and output waiting
//! for room. Cores run straight from the caller's input to its output, and
//! fall back on the [`Edge`] only around those bytes.

/// Undecided input and output waiting for room, `C` and `Q` sized for each
/// coroutine's longest undecided token.
#[derive(Clone, Debug)]
pub(crate) struct Edge<const C: usize, const Q: usize> {
    carry: [u8; C],
    carry_len: usize,
    queue: [u8; Q],
    start: usize,
    end: usize,
}

impl<const C: usize, const Q: usize> Default for Edge<C, Q> {
    fn default() -> Self {
        Self {
            carry: [0; C],
            carry_len: 0,
            queue: [0; Q],
            start: 0,
            end: 0,
        }
    }
}

impl<const C: usize, const Q: usize> Edge<C, Q> {
    /// Runs `core` over `src` into `dst`, returning the bytes read and
    /// written.
    ///
    /// `core` takes the input, the output and whether the input ends, and
    /// returns the bytes read, written and held. Held bytes sit past the
    /// written ones and go back ahead of the unread input. Unless the input
    /// ends, `core` reads no undecided token. When it stops short, the edge
    /// runs it over the carried bytes into the queue, so one byte of room
    /// always progresses.
    pub(crate) fn resume(
        &mut self,
        src: &[u8],
        dst: &mut [u8],
        mut core: impl FnMut(&[u8], &mut [u8], bool) -> [usize; 3],
    ) -> (usize, usize) {
        let mut read = 0;
        let mut written = 0;

        loop {
            let len = (self.end - self.start).min(dst.len() - written);
            dst[written..][..len].copy_from_slice(&self.queue[self.start..][..len]);
            self.start += len;
            written += len;

            if self.start < self.end {
                return (read, written);
            }

            if self.carry_len == 0 {
                let [r, w, held] = core(&src[read..], &mut dst[written..], false);
                read += r;
                written += w;
                self.carry[..held].copy_from_slice(&dst[written..][..held]);
                self.carry_len = held;
            }

            if read == src.len() {
                return (read, written);
            }

            let len = (C - self.carry_len).min(src.len() - read);
            let total = self.carry_len + len;
            self.carry[self.carry_len..total].copy_from_slice(&src[read..][..len]);
            read += len;

            let [r, w, held] = core(&self.carry[..total], &mut self.queue, false);
            self.carry.copy_within(r..total, held);
            self.carry[..held].copy_from_slice(&self.queue[w..][..held]);
            self.carry_len = held + total - r;
            self.start = 0;
            self.end = w;
        }
    }

    /// Ends the input: runs `core` over the carried bytes, then writes the
    /// queue and `core`'s output.
    ///
    /// Returns the bytes written, or none when `dst` is too short, changing
    /// nothing: the caller keeps `core`'s changes only on success.
    pub(crate) fn finish(
        &mut self,
        dst: &mut [u8],
        core: impl FnOnce(&[u8], &mut [u8]) -> usize,
    ) -> Option<usize> {
        let queued = self.end - self.start;
        let mut tail = [0; Q];
        let w = core(&self.carry[..self.carry_len], &mut tail);
        let dst = dst.get_mut(..queued + w)?;

        dst[..queued].copy_from_slice(&self.queue[self.start..self.end]);
        dst[queued..].copy_from_slice(&tail[..w]);
        *self = Self::default();
        Some(queued + w)
    }
}
