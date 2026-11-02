//! A default codec and the token the application installs, alone in its
//! binary since the token is the process's.

use quoted_printable_codec::{
    codec::QuotedPrintableCodec, cpu::QuotedPrintableCpu, decode::QuotedPrintableDecode,
};
use simd_cpu::SimdCpu;

#[test]
fn a_default_codec_runs_the_installed_token() {
    let before = QuotedPrintableCodec::default();
    assert_eq!(before, QuotedPrintableCodec::BASELINE);

    let cpu = SimdCpu::install_detected().unwrap();
    let after = QuotedPrintableCodec::default();
    let explicit = QuotedPrintableCodec {
        cpu: QuotedPrintableCpu::new(SimdCpu::NONE),
        ..QuotedPrintableCodec::default()
    };

    assert_eq!(after.cpu, QuotedPrintableCpu::new(cpu));
    assert_eq!(explicit.cpu, QuotedPrintableCpu::new(SimdCpu::NONE));
    assert_eq!(before, QuotedPrintableCodec::BASELINE);
    assert_eq!(
        QuotedPrintableCodec::BASELINE.cpu,
        QuotedPrintableCpu::new(SimdCpu::BASELINE)
    );

    let encoded = [before, after].map(|codec| {
        let mut out = [0; 256];
        let len = codec.encode("café\n".as_bytes(), &mut out).unwrap();
        out[..len].to_vec()
    });
    assert_eq!(
        encoded,
        [b"caf=C3=A9\r\n".to_vec(), b"caf=C3=A9\r\n".to_vec()]
    );

    let mut decoder = QuotedPrintableDecode::new(before);
    let mut out = [0; 16];
    let written = decoder.resume(b"caf=C3=A9", &mut out).written;
    assert_eq!(&out[..written], "café".as_bytes());
}
