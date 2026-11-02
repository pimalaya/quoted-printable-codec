//! # CPU token
//!
//! [`QuotedPrintableCpu`], the backend a codec runs, picked from a
//! [`SimdCpu`]: the caller's, or by default the installed one.

use simd_cpu::SimdCpu;

/// The backend a codec runs, by construction one the running CPU supports.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct QuotedPrintableCpu {
    backend: QuotedPrintableBackend,
}

impl QuotedPrintableCpu {
    /// The widest backend `cpu` supports: AVX-512 (VBMI, VBMI2, BMI1, BMI2,
    /// LZCNT, POPCNT), AVX2 (BMI1, LZCNT, POPCNT), NEON, then portable.
    pub const fn new(cpu: SimdCpu) -> Self {
        use simd_cpu::SimdFeature::{
            Avx2, Avx512Vbmi, Avx512Vbmi2, Bmi1, Bmi2, Lzcnt, Neon, Popcnt,
        };

        let bits = cpu.has(Bmi1) && cpu.has(Lzcnt) && cpu.has(Popcnt);
        let avx512 = cpu.has(Avx512Vbmi2) && cpu.has(Avx512Vbmi) && cpu.has(Bmi2);
        let backend = match (avx512 && bits, cpu.has(Avx2) && bits, cpu.has(Neon)) {
            #[cfg(target_arch = "x86_64")]
            (true, ..) => QuotedPrintableBackend::Avx512,
            #[cfg(target_arch = "x86_64")]
            (_, true, _) => QuotedPrintableBackend::Avx2,
            #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
            (.., true) => QuotedPrintableBackend::Neon,
            _ => QuotedPrintableBackend::Portable,
        };

        Self { backend }
    }

    /// The backend this token names.
    pub const fn backend(self) -> QuotedPrintableBackend {
        self.backend
    }
}

impl Default for QuotedPrintableCpu {
    /// The backend of [`SimdCpu::current`], the installed or baseline token.
    #[inline]
    fn default() -> Self {
        Self::new(SimdCpu::current())
    }
}

/// A backend of the codec.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum QuotedPrintableBackend {
    /// Safe 32-byte blocks the compiler vectorizes, on every target.
    Portable,
    /// AVX2 32-byte blocks, compressed through byte shuffles.
    #[cfg(target_arch = "x86_64")]
    Avx2,
    /// AVX-512 64-byte blocks, compressed by VBMI2.
    #[cfg(target_arch = "x86_64")]
    Avx512,
    /// NEON 16-byte blocks, compressed through table lookups.
    #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
    Neon,
}

/// The backends the running CPU supports, narrowest first.
#[cfg(test)]
pub(crate) fn available() -> alloc::vec::Vec<QuotedPrintableCpu> {
    let cpu = SimdCpu::detect();
    let narrow = cpu.without(simd_cpu::SimdFeature::Avx512Vbmi2);
    let mut cpus = alloc::vec::Vec::new();

    for cpu in [SimdCpu::NONE, narrow, cpu].map(QuotedPrintableCpu::new) {
        if !cpus.contains(&cpu) {
            cpus.push(cpu);
        }
    }

    cpus
}

#[cfg(test)]
mod tests {
    use simd_cpu::{SimdCpu, SimdFeature::*};

    use crate::cpu::{QuotedPrintableBackend, available};

    /// Fails where a backend the CPU has would go untested.
    #[test]
    fn every_backend_the_cpu_has_is_tested() {
        let cpu = SimdCpu::detect();
        let bits = [Bmi1, Lzcnt, Popcnt]
            .iter()
            .all(|&feature| cpu.has(feature));
        let backends: alloc::vec::Vec<_> = available().iter().map(|cpu| cpu.backend()).collect();
        let mut expected = alloc::vec![QuotedPrintableBackend::Portable];

        #[cfg(target_arch = "x86_64")]
        if cpu.has(Avx2) && bits {
            expected.push(QuotedPrintableBackend::Avx2);
        }

        #[cfg(target_arch = "x86_64")]
        if [Avx512Vbmi, Avx512Vbmi2, Bmi2]
            .iter()
            .all(|&feature| cpu.has(feature))
            && bits
        {
            expected.push(QuotedPrintableBackend::Avx512);
        }

        #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
        expected.push(QuotedPrintableBackend::Neon);

        assert_eq!(backends, expected, "{bits}");
    }
}
