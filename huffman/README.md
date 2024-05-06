# quake3-huffman-rs

Implementation of Huffman coding as implemented in the Quake 3 network protocol, both adaptive and (not yet) fixed.

# TODOs
- Replace `println!` with `log::debug!` or `tracing::debug!` or assertions
- Replace `unwrap` with `expect` and `Error`
- https://nnethercote.github.io/perf-book/
- Optimize structs (size, `NonZero`)
- https://rust-lang.github.io/api-guidelines/
- Make more generic (momo), hide dependencies
- https://www.lurklurk.org/effective-rust/
- Make `no_std`
- Use `const` where possible
- Add rustdoc, `must_use` etc.
- Build rustdoc for GitHub Pages
- Add assertions
- Add benchmarks for encode, decode and adaptive ✔️, fixed ❌
- Add fuzzing
- https://github.com/rust-secure-code/safety-dance
- GitHub Actions CI
- Publish to crates.io
