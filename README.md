# FRAT-XOR

The frat-xor is a toolchain for translating CNF-XOR proof from FRAT-XOR to XLRUP format. The format description is [here](https://github.com/meelgroup/frat-xor/blob/main/format.md).

## Build

You must have the `rust` toolchain, including `cargo`, installed. Then you can build with:

```
cargo clean
cargo build --release
```

You can find the compiled tool at `./target/release/frat-xor`. In case you need a static compilation, build with:

```
cargo clean
rustup target add x86_64-unknown-linux-musl
cargo build --release --target=x86_64-unknown-linux-musl
```

The compiled tool is at `./target/x86_64-unknown-linux-musl/release/frat-xor`.

## Usage

The following command elaborates the unsatisfiability proof in FRAT-XOR format (`xfrat_file`) of the CNF-XOR formula (`xnf_file`)
  and produces the proof in XLRUP format (`xlrup_file`).

```
frat-xor elab xfrat_file xnf_file xlrup_file
```

You can find an example below.

```
frat-xor elab ./example/test_1.xfrat ./example/test_1.xnf ./example/test_1.xlrup
```
