# FRAT-XOR

This repository contains the `frat-xor` tool for elaborating unsatisfiability proofs for CNF-XOR formulas from FRAT-XOR to XLRUP format. The latter format is supported by a formally verified proof checker `cake_xlrup`, whose verified binary implementation can be found in its own [directory](cake_xlrup/).
The format description is [here](format.md).

This repository was created as a fork of `FRAT-rs` which is available [here](https://github.com/digama0/frat).

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

The elaborated proof can be checked with `cake_xlrup`.

```
cake_xlrup ./example/test_1.xnf ./example/test_1.xlrup
```
