# quake3-huffman-rs

[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

Implementation of Huffman coding as implemented in the Quake 3 network protocol, both adaptive and (not yet) fixed.

# TODOs
- Replace `println!` with `log::debug!` or `tracing::debug!` or assertions
- https://nnethercote.github.io/perf-book/
- Optimize structs (size, `NonZero`)
- https://rust-lang.github.io/api-guidelines/
- Make more generic (momo), hide dependencies
- https://www.lurklurk.org/effective-rust/
- Make `no_std`
- Add rustdoc, `must_use` etc.
- Build rustdoc for GitHub Pages
- Add assertions
- Add benchmarks for encode, decode and adaptive ✔️, fixed ❌
- GitHub Actions CI
- Publish to crates.io

```console
$ # explicit `perf` CLI path is needed for WSL2, this does not match `uname --kernel-release`
$ PERF=/usr/lib/linux-tools-5.15.0-105/perf cargo flamegraph --bench decode
$ $BROWSER flamegraph.svg
```

```console
$ cargo +nightly fuzz run decode-adaptive
$ cargo +nightly fuzz run encode-adaptive
```
