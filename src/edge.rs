//! # Call edges
//!
//! What a coroutine keeps between two calls: the input bytes it could not
//! decide yet, and the output the caller's buffer had no room for. Both
//! coroutines run their core straight from the caller's input into the
//! caller's output, and fall back on the [`Edge`] only around those bytes.

/// The input bytes a core leaves undecided and the output waiting for room,
/// `C` and `Q` sized by each coroutine for its longest undecided token.
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
    /// `core` takes the input, the output and whether the input ends there,
    /// and returns the bytes it read and wrote and the bytes it held: output
    /// bytes it wrote after the ones it counts, which go back to the input
    /// ahead of the unread rest. Unless the input ends, it reads no token it
    /// cannot decide yet. When `core` stops short, the edge carries what is
    /// left with the next input bytes and runs `core` over them into its
    /// queue, so that one byte of room always progresses.
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

    /// Ends the input: runs `core` over the carried bytes, and writes what
    /// is queued and what `core` gives.
    ///
    /// Returns the bytes written, or none, changing nothing, when `dst` is
    /// too short: the caller keeps what `core` changed only on success.
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
