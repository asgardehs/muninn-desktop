# TODO

Running list of known follow-ups that aren't tied to a specific
open Phase of the rewrite. Phase scope lives in
[docs/rust-rewrite.md](docs/rust-rewrite.md).

## Licensing

- [ ] **Add LICENSE and NOTICE.** The NOTICE must name
  [Rhai](https://rhai.rs) (scripting engine) and
  [harper-core](https://writewithharper.com/) (grammar checker) at
  minimum. Sweep the full dependency tree when writing it —
  `cargo about` or `cargo-deny` can generate a complete list of crates
  and their licenses.
