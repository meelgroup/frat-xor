# CNF-XOR-BNN proof format

This file gives a provisional spec for a CNF proof format extended with theories (currently, XOR and BNN reasoning).

It is based on an extension of FRAT (RUP) with support for XOR and BNN reasoning.

## Overview

The FRAT-XOR-BNN proof format supports FRAT-style proofs extended with theory reasoning; the current design is meant to be easily emitted by CNF-XOR-BNN solvers.

Correspondingly, XLRUP is an elaborated proof format supported by the `cake_xlrup` verified proof checker.

Conversion from FRAT-XOR-BNN to XLRUP is supported by the `frat-xor` tool.

Our extensions follow a suggestion from the FRAT paper: "... it could pass the new methods on to some XLRAT backend format that understands these steps natively".

In particular, the XOR and BNN reasoning steps are *only* checked by `cake_xlrup`, but are mostly passed through unchanged by the elaborator (except for doing some bookkeeping).

Note, however, that we do not support RAT steps.

## Input CNF-XOR-BNN File Format

The input CNF-XOR-BNN file format is an extension of CNF DIMACS as supported by the CryptoMiniSat solver.

### Comments

Comment lines start with `c`.
They can appear anywhere in the file and are always ignored.

```
c this is a comment
```

### Header

The file (after stripping comments) must start with a header line:

```
p cnf #num_vars #num_clauses_and_xors_and_bnns
```

`#num_vars` is the maximum variable in use (variables are numbered `1,2,...,num_vars`).

`#num_clauses_and_xors_and_bnns` is the total number of CNF clauses, XOR constraints, and BNN constraints (henceforth, XORs and BNNs).

Subsequent lines must be in one of two formats.

### Clauses, XORs, and BNNs

Syntactically, clauses and XORs are both represented by lists of non-zero integers,

```
CLAUSE, XOR ::= { list of non-zero integers }
```

A BNN constraint is represented by a list of non-zero integers followed by zero and two non-zero integers; the latter non-zero integer is optional.

```
BNN ::= { list of non-zero integers } 0 cutoff [optional: output_lit]
```

To list a clause in the input file, write a line with a clause followed by a zero:

```
CLAUSE_LINE ::= CLAUSE 0
```

For example, the line `1 -2 3 0` represents the clause `x_1 OR NOT(x_2) OR x_3`.

To indicate an XOR in the input file, start the line with `x` followed by the constraint and end it with zero:

```
XOR_LINE ::= x { list of non-zero integers } 0
```

For example, the line `x 1 -2 3 0` represents the XOR `x_1 XOR NOT(x_2) XOR x_3 = 1`.

To represent a BNN in the input file, start the line with `b` followed by the constraint and end it with zero:

```
BNN_LINE ::= b { list of non-zero integers } 0 cutoff [optional: output_lit] 0
```

For example, the line `b 1 2 3 0 3 4 0` represents the BNN constraint `x_1 + x_2 + x_3 >= 3 <-> x_4`.

If the output literal is omitted, then the BNN constraint reduces to an at-most-K constraint, i.e., `b 1 2 3 0 3 0` represents the constraint `x_1 + x_2 + x_3 >= 3`.

However, the current CMS does not handle the case properly when the output_lit is missing in the input BNN. We will resolve this issue and add the cardinality constraint extension to the format.

## Proof Format

Throughout the following, the proof formats are allowed to explicitly refer to positive integer clause, XOR, or BNN IDs.

It will be clear from the context what kind of ID is being used, but for clarity, we indicate either `CID` (for clauses), `XID` (for XORs), or `BID` (for BNNs).

```
ID, CID, XID, BID    ::= ( single positive integer )
IDs, CIDs, XIDs, BIDs ::= { list of respective IDs }
```

### FRAT-XOR-BNN Format

The supported clausal steps are identical to FRAT although RAT and PR steps are
not supported.  The most important clause-only steps are as follows:

- Indicate an original clause and give it the `CID` identifier.

```
CLAUSE_ORIG_STEP ::= o CID CLAUSE 0
```

- Add a new clause (with optional hints `l CIDs`)

```
CLAUSE_ADD_STEP ::= a CID CLAUSE 0 l CIDs 0
```

- Delete a CLAUSE at the given ID.

```
CLAUSE_DEL_STEP ::= d CID CLAUSE 0
```

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

Additional BNN reasoning is supported as follows: 

- Indicate an original BNN and give it the `BID` identifier.
```
BNN_ORIG_STEP ::= o b BID lits 0 k cutoff output_lit 0
```

- Delete an BNN at the given ID.

```
BNN_DEL_STEP ::= d b BID lits 0 k cutoff output_lit 0
```

- Add a new clause implied by the indicated BNN constraint at BID with unit propagations from CIDs.

```
CLAUSE_FROM_BNN_STEP ::= i CID CLAUSE 0 b l BID 0 u CIDs 0
```

- Add a new BNN derived from an existing BNN with unit propagations from CIDs.

```
BNN_UPDATE_STEP ::= a b BID BNN 0 l BID CIDs 0
```

- Indicate a final BNN (currently, these steps are not checked).

```
BNN_FINAL_STEP ::= f b BID lits 0 k cutoff output_lit 0
```


### XLRUP Format

The elaborated XLRUP format is identical to LRUP with XOR additions.

For example, to add a clause by RUP, write the ID, list the clause, and finish with clausal unit propagation hints.

```
RUP_STEP ::= CID CLAUSE 0 CIDs 0
```

For clauses and BNN constraints, the inputs are given IDs immediately in their respective namespaces in order of appearance.

For XOR reasoning, note that *unlike* clauses and BNN, the XORs are *not* given IDs immediately.

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
XOR_DEL_STEP ::= x d XIDs 0
```

- Clauses can be derived from XORs (`i` stands for "implies"; `cx` means clause-from-XOR)

```
CLAUSE_FROM_XOR_STEP ::= i cx CID CLAUSE 0 XIDs 0
```

- XORs can be derived from clauses (`i x` stands for "implies XOR")

```
XOR_FROM_CLAUSE_STEP ::= i x XID XOR 0 CIDs 0
```

BNN reasoning behaves like a simpler version of XOR reasoning.

- Indicate an original BNN and give it the `BID` identifier.

```
BNN_ORIG_STEP ::= o b BID BNN 0
```

- BNN deletion

```
BNN_DEL_STEP ::= b d BIDs 0
```

- Clauses can be derived from BNN and unit clauses (`i` stands for "implies"; `cb` means clause-from-BNN)

```
CLAUSE_FROM_BNN_STEP ::= i cb CID CLAUSE 0 BID u CIDs 0
```

- BNN can be derived from BNN and unit clauses

```
BNN_ADD_STEP := b BID BNN 0 BID CIDs 0
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
