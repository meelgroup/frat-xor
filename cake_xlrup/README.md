# cake_xlrup

This folder contains a pre-compiled binary for `cake_xlrup` which checks the XLRUP format detailed in `../format.md`.
Proofs are read in the binary encoding detailed in `BINARY_FORMAT.md` (tentative, may change) by default, and in the text format with `--no-binary`.

Source and proof files are available in the main CakeML repository (https://github.com/CakeML/cakeml/tree/master/examples/cnf/xlrup)

The files are built from the following repository versions

```
CakeML: 0f1aca3913a175c390f0fd0b3af04f43d49a0fde
HOL4:   652d1f0aad5cd537ed1e5d470ba3444e70ba470e
```

# News

- 2026-10-23: the new version of `cake_xlrup` no longer accepts BNN steps. It has also gained a new API which is not backwards compatible (see documentation).
  The last version that checks BNN steps is `cake_xlrup` at commit `a1962afd35303d9d9de1a71e580229f707327c97` on `main`.
  Support may be recovered in the future.

# Instructions

Running `make` will build the proof checker (default 4GB heap/stack).

```
Usage:  cake_xlrup [--binary|--no-binary] [--] <CNF-XOR formula file> <optional: XLRUP proof file>
        cake_xlrup --dump [--] <CNF-XOR formula file>

Run XLRUP unsatisfiability proof checking.
The proof is read from the proof file if one is given, and from standard input otherwise.
The proof is read in binary format unless --no-binary is given.
--dump parses the formula file and prints the formula.
```

Example usage:

```
./cake_xlrup example.xnf example.xlrupb

s VERIFIED UNSAT

./cake_xlrup --no-binary example.xnf example.xlrup

s VERIFIED UNSAT
```

The heap and stack sizes (in MiB) can be changed at compile time with `-DCML_HEAP_SIZE=<n>` and `-DCML_STACK_SIZE=<n>` in the `Makefile`, or at run time with flags placed before the program arguments:

```
./cake_xlrup --CML_HEAP_SIZE=16384 --CML_STACK_SIZE=8192 example.xnf example.xlrupb
```

`./cake_xlrup --h` lists these flags. We recommend giving more heap for proof checking if your system memory allows for it.

*NOTE: unit-clause hints on XOR additions (`u CIDs`) are experimental; `BINARY_FORMAT.md` §4 gives the semantics the checker implements.*
