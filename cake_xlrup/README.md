# cake_xlrup

This folder contains a pre-compiled binary for `cake_xlrup` which checks the XLRUP format detailed in `format.md`.

Source and proof files are available in the main CakeML repository (https://github.com/CakeML/cakeml/tree/master/examples/xlrup_checker)

The files are built from the following repository versions

```
HOL4: 37389f39aa1f331637b2525ad92fdba43703a13e

CakeML: e1650fc504837c0fbd3931cc5066914ffdc9d877
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

The default heap/stack sizes are set to 4GB. There are three ways to modify the default values:

1) Directly modify the values of `cml_heap_sz` and `cml_stack_sz` in `basis_ffi.c`.

2) Pass the appropriate flags, e.g., `-DCML_HEAP_SIZE=65536` `-DCML_STACK_SIZE=16384` at compile time.

3) Set the environment variables at run time:

  ```
  export CML_HEAP_SIZE=1234
  export CML_STACK_SIZE=5678
  ./cake_xlrup ... 
  ```

We recommend giving more heap for proof checking if your system memory allows for it.
