---
cairn: tasks
change: a-shared-cpu-token
---

- [x] Wait for the AVX-512 pass to land (perfs pass 4, 72d8dc6)
- [x] simd-cpu dependency, `std` feature removed
- [x] `QuotedPrintableCpu::new(SimdCpu)` checking BMI1 too, the old constructors removed
- [x] Tests and bench on `SimdCpu`
- [x] Release assembly compared with the tree before
- [x] Docs, README, Cargo and nix descriptions, the-wider-reach handoff
- [x] Spec folded, log entry written
