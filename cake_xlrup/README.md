# cake_xlrup

*NOTE: currently, this dev version only does XORs (no BNN support)

This folder contains a pre-compiled binary for `cake_xlrup` which checks the XLRUP format detailed in `format.md`.

Source and proof files are available in the main CakeML repository (https://github.com/CakeML/cakeml/tree/master/examples/xlrup_checker)

The files are built from the following repository versions

```
CakeML: a538cdd71683afc4b996e876921b71fb82e53244
HOL4:   dfff742b1f89a43e9c1812a146e9a40d4c4d89fc
```

# Instructions

Running `make` will build the proof checker (default 4GB heap/stack).

```
Usage:  cake_xlrup <formula file> <optional: XLRUP proof file>

Run XLRUP unsatisfiability proof checking (if proof is given)
```

Example usage:

```
./cake_xlrup example.xnf example.xlrup 

s VERIFIED UNSAT
```
