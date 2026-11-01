//! # CPU token
//!
//! [`QuotedPrintableCpu`], the backend a
//! [`QuotedPrintableCodec`](crate::codec::QuotedPrintableCodec) runs: the
//! application picks it, the library never detects CPU features.

/// The backend a codec runs, by construction one the running CPU supports.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct QuotedPrintableCpu {
    backend: QuotedPrintableBackend,
}

impl QuotedPrintableCpu {
    /// The best backend the build's target features guarantee: portable on
    /// a default build.
    pub const BASELINE: Self = Self {
        backend: match AVX512 {
            #[cfg(target_arch = "x86_64")]
            true => QuotedPrintableBackend::Avx512,
            _ => QuotedPrintableBackend::Portable,
        },
    };

    /// Names `backend` without checking the running CPU.
    ///
    /// # Safety
    ///
    /// The running CPU must support the instructions of `backend`.
    pub const unsafe fn assume(backend: QuotedPrintableBackend) -> Self {
        Self { backend }
    }

    /// The best backend the running CPU supports, detected at run time.
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        if detect_avx512() {
            return Self {
                backend: QuotedPrintableBackend::Avx512,
            };
        }

        Self::BASELINE
    }

    /// The backend this token names.
    pub const fn backend(self) -> QuotedPrintableBackend {
        self.backend
    }

    /// Whether the AVX-512 blocks encode `src`: on their backend, unless the
    /// input is short with few bytes to escape.
    pub(crate) fn wide(self, src: &[u8]) -> bool {
        match self.backend {
            #[cfg(target_arch = "x86_64")]
            // SAFETY: the token names a backend the CPU supports.
            QuotedPrintableBackend::Avx512 => unsafe { crate::avx512::dense(src) },
            _ => false,
        }
    }
}

impl Default for QuotedPrintableCpu {
    fn default() -> Self {
        Self::BASELINE
    }
}

/// A backend of the codec.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum QuotedPrintableBackend {
    /// Safe blocks of 32 bytes the compiler vectorizes, on every target.
    Portable,
    /// AVX-512 blocks of 64 bytes with the VBMI2 byte compress, and BMI1 and
    /// BMI2.
    #[cfg(target_arch = "x86_64")]
    Avx512,
}

/// Whether the build target guarantees what the AVX-512 backend runs.
const AVX512: bool = cfg!(all(
    target_feature = "avx512vbmi2",
    target_feature = "avx512vbmi",
    target_feature = "bmi1",
    target_feature = "bmi2",
    target_feature = "lzcnt",
    target_feature = "popcnt",
));

#[cfg(all(target_arch = "x86_64", any(feature = "std", test)))]
fn detect_avx512() -> bool {
    use std::arch::is_x86_feature_detected as has;

    has!("avx512vbmi2")
        && has!("avx512vbmi")
        && has!("bmi1")
        && has!("bmi2")
        && has!("lzcnt")
        && has!("popcnt")
}

/// The backends the running CPU supports.
#[cfg(test)]
pub(crate) fn available() -> alloc::vec::Vec<QuotedPrintableCpu> {
    let mut cpus = alloc::vec![QuotedPrintableCpu {
        backend: QuotedPrintableBackend::Portable,
    }];

    #[cfg(target_arch = "x86_64")]
    if detect_avx512() {
        cpus.push(QuotedPrintableCpu {
            backend: QuotedPrintableBackend::Avx512,
        });
    }

    cpus
}
