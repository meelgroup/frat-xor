# CNF-XOR proof format

This file gives a provisional spec for a CNF-XOR proof format based on an extension of FRAT for RUP and XOR reasoning.

## Overview

The FRAT-XOR proof format supports FRAT-style proofs extended with XOR reasoning; this format is designed to be easily emitted by CNF-XOR solvers.

Correspondingly, XLRUP is an elaborated proof format supported by the `cake_xlrup` verified proof checker.

Conversion from FRAT-XOR to XLRUP is supported by the `frat-xor` tool.

Our extensions follow a suggestion from the FRAT paper: "... it could pass the new methods on to some XLRAT backend format that understands these steps natively".

In particular, the XOR reasoning steps are *only* checked by `cake_xlrup`, but are mostly passed through unchanged by the elaborator (except for doing some bookkeeping).

## Input CNF-XOR File Format

The input CNF-XOR file format is an extension of CNF DIMACS as supported by the CryptoMiniSat solver.

### Comments

Comment lines start with `c`.
They can appear anywhere in the file and are always ignored.

```
c this is a comment
```

### Header

The file (after stripping comments) must start with a header line:

```
p cnf #num_vars #num_clauses_and_xors
```

`#num_vars` is the maximum variable in use (variables are numbered `1,2,...,num_vars`).

`#num_clauses_and_xors` is total number of CNF clauses and XOR constraints (henceforth, XORs).

Subsequent lines must be in one of two formats.

### Clauses and XORs

Syntactically, clauses and XORs are both represented by lists of non-zero integers.

```
CLAUSE, XOR ::= { list of non-zero integers }
```

To list a clause in the input file, write a line with a clause followed by a zero:

```
CLAUSE_LINE ::= CLAUSE 0
```

For example the line `1 -2 3 0` represents the clause `x_1 OR NOT(x_2) OR x_3`.

To indicate an XOR in the input file, start the line with `x` followed by the constraint and end it with zero:

```
XOR_LINE ::= x { list of non-zero integers } 0
```

For example the line `x 1 -2 3 0` represents the XOR `x_1 XOR NOT(x_2) XOR x_3 = 1`.

## Proof Format

Throughout the following, the proof formats are allowed to explicitly refer to positive integer clause or XOR IDs.

It will be clear from the context what kind of ID is being used, but for clarity, we indicate either `CID` (for clauses) or `XID` (for XORs).

```
ID, CID, XID    ::= ( single positive integer )
IDs, CIDs, XIDs ::= { list of respective IDs }
```

### FRAT-XOR Format

The supported clausal steps are identical to FRAT although RAT and PR steps are not supported.

Additional XOR reasoning is supported as follows:

- Indicate an original XOR and give it the `XID` identifier.

```
XOR_ORIG_STEP ::= o x XID XOR 0
```

- Add a new XOR derived from other XORs (indicated by IDs) by XOR addition.

```
XOR_ADD_STEP ::= a x XID XOR 0 l XIDs 0
```

- Delete an XOR at the given ID.

```
XOR_DEL_STEP ::= d x XID XOR 0
```

- Add a new clause implied by adding the indicated XORs.

```
CLAUSE_FROM_XOR_STEP ::= i CID CLAUSE 0 l XIDs 0
```

- Add a new XOR implied by the indicated clauses.

```
XOR_FROM_CLAUSE_STEP ::= i x XID XOR 0 l CIDs 0
```

- Indicate a final XOR (currently, these steps are not checked).

```
XOR_FINAL_STEP ::= f x XID XOR 0
```

### XLRUP Format

The elaborated XLRUP format is identical to LRUP with XOR additions.

For example, to add a clause by RUP, write the ID, list the clause, and finish with clausal unit propagation hints.

```
RUP_STEP ::= CID CLAUSE 0 CIDs 0
```

For XOR reasoning, note that *unlike* clauses, the XORs are *not* given IDs immediately.

Otherwise, the steps are largely identical to FRAT-XOR with minor syntactic differences.

- Indicate an original XOR and give it the `XID` identifier.

```
XOR_ORIG_STEP ::= o x XID XOR 0
```

- XORs can be derived from other XORs by addition.

```
XOR_ADD_STEP ::= x XID XOR 0 XIDs 0
```

- XOR deletion

```
XOR_DEL_STEP ::= x d XIDs
```

- Clauses can be derived from XORs (`i` stands for "implies")

```
CLAUSE_FROM_XOR_STEP ::= i CID CLAUSE 0 XIDs 0
```

- XORs can be derived from clauses (`i x` stands for "implies XOR")

```
XOR_FROM_CLAUSE_STEP ::= i x XID XOR 0 CIDs 0
```

### Experimental

The checkers support slightly more powerful XOR addition steps with builtin unit propagation.

The semantics of such a step is to add all the constraints indicated by `XIDs`, then unit propagate `CIDs` on the result.

```
FRAT_XOR_XOR_ADD_STEP ::= a x XID XOR 0 l XIDs 0 u CIDs 0
```

The unit propagations are listed similarly for XLRUP.

```
XLRUP_XOR_ADD_STEP ::= x XID XOR 0 XIDs u CIDs 0
```
