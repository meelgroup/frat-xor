# cake_xlrup

*NOTE: currently, this dev version only does XORs (no BNN support)

This folder contains a pre-compiled binary for `cake_xlrup` which checks the XLRUP format detailed in `../format.md`.
Proofs are read in the binary encoding detailed in `BINARY_FORMAT.md` (tentative, may change) by default, and in the text format with `--no-binary`.

Source and proof files are available in the main CakeML repository, currently under development (https://github.com/CakeML/cakeml/tree/xlrup-new/examples/cnf/xlrup)

The files are built from the following repository versions

```
CakeML: 2aecfb35c41530fc4c4e03b5da725c7474cddf56
HOL4:   652d1f0aad5cd537ed1e5d470ba3444e70ba470e
```

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

*NOTE: unit-clause hints on XOR additions (`u CIDs`) are experimental; `BINARY_FORMAT.md` §4 gives the semantics the checker implements.*
