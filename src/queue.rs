//! # Output queue
//!
//! The few bytes a step produces before the caller's buffer takes them, so
//! that a coroutine progresses with any room, a single byte included.

/// A fixed-size queue of output bytes, filled empty and drained in order.
#[derive(Clone, Debug)]
pub(crate) struct Queue<const N: usize> {
    bytes: [u8; N],
    start: usize,
    end: usize,
}

impl<const N: usize> Queue<N> {
    pub(crate) const fn new() -> Self {
        Self {
            bytes: [0; N],
            start: 0,
            end: 0,
        }
    }

    pub(crate) const fn len(&self) -> usize {
        self.end - self.start
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Appends `bytes`; the caller sizes `N` for its longest step.
    pub(crate) fn push(&mut self, bytes: &[u8]) {
        if self.is_empty() {
            self.start = 0;
            self.end = 0;
        }

        self.bytes[self.end..self.end + bytes.len()].copy_from_slice(bytes);
        self.end += bytes.len();
    }

    /// Moves as many queued bytes as fit into `dst`, returning how many.
    pub(crate) fn drain(&mut self, dst: &mut [u8]) -> usize {
        let len = self.len().min(dst.len());
        dst[..len].copy_from_slice(&self.bytes[self.start..self.start + len]);
        self.start += len;
        len
    }
}
