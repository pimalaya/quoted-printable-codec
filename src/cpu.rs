//! # CPU token
//!
//! [`QuotedPrintableCpu`], the backend a
//! [`QuotedPrintableCodec`](crate::codec::QuotedPrintableCodec) runs, picked
//! from the [`SimdCpu`] the application hands over: the library never detects
//! CPU features.

use simd_cpu::SimdCpu;

/// The backend a codec runs, by construction one the running CPU supports.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct QuotedPrintableCpu {
    backend: QuotedPrintableBackend,
}

impl QuotedPrintableCpu {
    /// The AVX-512 backend when `cpu` has AVX-512 VBMI and VBMI2, BMI1, BMI2,
    /// LZCNT and POPCNT, the portable one otherwise.
    pub const fn new(cpu: SimdCpu) -> Self {
        Self {
            backend: widest(cpu),
        }
    }

    /// The backend this token names.
    pub const fn backend(self) -> QuotedPrintableBackend {
        self.backend
    }

    /// Whether the AVX-512 blocks encode `src`: on their backend, unless the
    /// input is short with few bytes to escape.
    #[cfg_attr(not(target_arch = "x86_64"), expect(unused_variables))]
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
        const { Self::new(SimdCpu::BASELINE) }
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

#[cfg(target_arch = "x86_64")]
const fn widest(cpu: SimdCpu) -> QuotedPrintableBackend {
    use simd_cpu::SimdFeature::{Avx512Vbmi, Avx512Vbmi2, Bmi1, Bmi2, Lzcnt, Popcnt};

    if cpu.has(Avx512Vbmi2)
        && cpu.has(Avx512Vbmi)
        && cpu.has(Bmi1)
        && cpu.has(Bmi2)
        && cpu.has(Lzcnt)
        && cpu.has(Popcnt)
    {
        QuotedPrintableBackend::Avx512
    } else {
        QuotedPrintableBackend::Portable
    }
}

#[cfg(not(target_arch = "x86_64"))]
const fn widest(_: SimdCpu) -> QuotedPrintableBackend {
    QuotedPrintableBackend::Portable
}

/// The backends the running CPU supports, portable first.
#[cfg(test)]
pub(crate) fn available() -> alloc::vec::Vec<QuotedPrintableCpu> {
    let portable = QuotedPrintableCpu::new(SimdCpu::NONE);
    let detected = QuotedPrintableCpu::new(SimdCpu::detect());
    let mut cpus = alloc::vec![portable];

    if detected != portable {
        cpus.push(detected);
    }

    cpus
}
