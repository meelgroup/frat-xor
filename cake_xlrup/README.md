# cake_xlrup

*NOTE: currently, this dev version only does XORs (no BNN support)

This folder contains a pre-compiled binary for `cake_xlrup` which checks the XLRUP format detailed in `format.md`.
Proofs are read in the binary encoding detailed in `BINARY_FORMAT.md` (tentative, may change) by default, and in the text format with `--no-binary`.

Source and proof files are available in the main CakeML repository (https://github.com/CakeML/cakeml/tree/xlrup-new/examples/cnf/xlrup)

The files are built from the following repository versions

```
CakeML: 05173a4863bdef516f8c4c7eaf956d188acf2e40
HOL4:   dfff742b1f89a43e9c1812a146e9a40d4c4d89fc
```

# Instructions

Running `make` will build the proof checker (default 4GB heap/stack).

```
Usage:  cake_xlrup [--binary|--no-binary] [--] <CNF-XOR formula file> <optional: XLRUP proof file>

Run XLRUP unsatisfiability proof checking (if proof is given)
The proof file is read in binary format unless --no-binary is given.
```

Example usage:

```
./cake_xlrup example.xnf example.xlrupb

s VERIFIED UNSAT

./cake_xlrup --no-binary example.xnf example.xlrup

s VERIFIED UNSAT
```

*NOTE: unit-clause hints on XOR additions (`u CIDs`) are experimental; `BINARY_FORMAT.md` §4 gives the semantics the checker implements.*
