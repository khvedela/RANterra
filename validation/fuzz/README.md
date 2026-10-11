# Isolated protocol fuzz validator

No runtime dependency or network service. The fuzzer calls the same bounded Rust parsers, including NAS/N1, NGAP setup success/failure, release, GTP-U, IPv4/ICMP and AUTS verification. Panics, sanitizer findings, excessive RSS or a >5 s single input fail the run. Seeds contain synthetic laboratory wire messages only.

```sh
rustup toolchain install nightly-2026-07-20
cargo +nightly-2026-07-20 install cargo-fuzz --version 0.13.2 --locked
sh scripts/fuzz-protocols.sh 300
```

The separate lockfile preserves the application's runtime package versions. Six tooling dependencies are added: libfuzzer-sys 0.4.10, arbitrary 1.5.0, cc 1.7.0, find-msvc-tools 0.1.14, jobserver 0.1.35 and shlex 2.0.1. Select MIT for the dual MIT/Apache-2.0 packages; libfuzzer-sys additionally contains LLVM libFuzzer under NCSA. These permissive licenses are compatible with the MIT application; this harness is not embedded in production assets. Preserve the crate MIT/Apache and bundled LLVM copyright notices if redistributing the tooling binaries. No AGPL source is included. The normal license checker includes this isolated graph; cargo audit also scans its lockfile.

Coverage guidance is instrumented; input generation is seeded, but execution counts depend on wall-clock duration and platform. A passing bounded campaign is evidence for the exercised paths, not exhaustive parser verification or a proof of memory-safety of native dependencies. Keep crash artifacts for minimization and regression tests; do not convert crashes into ignored errors.
