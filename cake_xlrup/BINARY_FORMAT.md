# Binary XLRUP

**Status: tentative.** This encoding belongs to the development version of
`cake_xlrup` and may still change. CryptoMiniSat does not write it yet.

`cake_xlrup` reads XLRUP proofs in this binary encoding by default, and in the
text encoding of [`../format.md`](../format.md) with `--no-binary`. This file
gives the bytes of the binary encoding and what the checker does with each step.
Each text step corresponds to exactly one binary record, and back. Neither
encoding allows comments: in a text proof, a comment line or a blank line is
rejected.

## 1. Numbers

All numbers use the variable-byte codec of binary DRAT, LRAT and FRAT:

- **Unsigned numbers** are written 7 bits per byte, least significant group
  first, with the high bit `0x80` set on every byte except the last.
  For example, 5 is `05` and 300 is `ac 02`.
- **An id** `n` (of a clause or an XOR) is written as the number `2n`.
- **A literal** `l` is written as `2|l| + (1 if l < 0 else 0)`: `x3` is `06`,
  `¬x3` is `07`.
- **A list** of ids or literals is its elements followed by one `00` byte.

Numbers must use the shortest encoding. Then a `00` byte never occurs inside a
number, since every byte but the last has `0x80` set and the last byte of a
non-zero number is not zero. So the checker finds the end of a list without
decoding it. A longer encoding always ends in `00`, which the checker takes as
the end of the list.

## 2. Records

A record starts with a tag: one byte for clause steps, two for XOR steps (`x`
then a kind byte). The rest is laid out as in the table below. A deletion record
is just its list of ids. Every other record has its own id, its literal list,
then its hint lists. Nothing separates these parts beyond the `00` that ends
each list, and the next record starts right after the last one.

| step | text | binary |
|---|---|---|
| RUP clause | `CID CLAUSE 0 CIDs 0` | `a` id(CID) lit\* `00` id\*(CIDs) `00` |
| delete clauses | `CID d CIDs 0` | `d` id\*(CIDs) `00` |
| original XOR | `o x XID XOR 0` | `x` `o` id(XID) lit\* `00` |
| XOR addition | `x XID XOR 0 XIDs [u CIDs] 0` | `x` `a` id(XID) lit\* `00` id\*(XIDs) `00` id\*(CIDs) `00` |
| delete XORs | `x d XIDs 0` | `x` `d` id\*(XIDs) `00` |
| clause from XORs | `i cx CID CLAUSE 0 XIDs 0` | `x` `c` id(CID) lit\* `00` id\*(XIDs) `00` |
| XOR from clauses | `i x XID XOR 0 CIDs 0` | `x` `i` id(XID) lit\* `00` id\*(CIDs) `00` |

The tag bytes are ASCII: `a` = `61`, `d` = `64`, `x` = `78`. So are the kind
bytes: `o` = `6f`, `a` = `61`, `d` = `64`, `c` = `63`, `i` = `69`.

- The `a` and `d` records are those of binary LRAT, so a proof without XOR
  steps is a binary LRAT proof with RUP steps only.
- The XOR addition record **always has three lists**. When the step has no
  unit-clause hints, the third list is the single byte `00`.
- The text deletion line starts with a `CID` that the checker ignores. The
  binary `d` record has no such id; its list holds every id to delete.

## 3. What the checker does

The checker keeps two databases: clauses by `CID`, and XORs by `XID`. The
formula's clause lines get the `CID`s `1, 2, …` in the order they appear. Its
XOR lines get no id; an original XOR enters the XOR database only through an
`x o` step. An XOR `l1 ⊕ … ⊕ lk` means that an odd number of its literals are
true.

Steps are checked in order, and the first step that fails stops the run.

- **`a` — RUP.** The clause is stored at `CID` if unit propagation refutes its
  negation using the hint clauses, in the order given. Under the current
  assignment each hint must either become unit, and its remaining literal is
  made true, or become false, which ends the check successfully. Running out of
  hints without a conflict fails. The empty clause is written as `a` id `00`
  hints `00`.
- **`d` — delete clauses.** Ids that name no clause are ignored.
- **`x o` — original XOR.** The XOR must be one of the formula's XOR lines,
  **with the same literals in the same order**, and it is stored at `XID`. The
  same XOR with its literals reordered is rejected (`unable to find original XOR`).
- **`x a` — XOR addition.** The XORs of the first hint list are added to the
  claimed XOR, and the unit clauses of the second list are substituted into
  the sum (see §4). The result must be the empty XOR (`0 = 0`). On success the
  claimed XOR is stored at `XID`.
- **`x d` — delete XORs.** Ids that name no XOR are ignored.
- **`x c` — clause from XORs.** The hint XORs must add up to exactly the XOR
  whose literals are those of the clause: `l1 ⊕ … ⊕ lk` for the clause
  `l1 ∨ … ∨ lk`. That XOR implies the clause. The clause is stored at `CID`.
- **`x i` — XOR from clauses.** Every clause of the XOR's CNF encoding (the
  `2^(k−1)` clauses over its `k` literals with an even number of them negated)
  must contain one of the hint clauses. On success the XOR is stored at `XID`.
  The cost is exponential in `k`.

A step that stores at an id already in use replaces what was there. After the
last record, the formula is reported unsatisfiable if the clause database
contains the empty clause.

## 4. ⚠ Unit-clause hints on XOR addition

**Status.** The checker accepts them, and its correctness proof covers them like
every other step. They are experimental, though: CryptoMiniSat's XLRUP writer
does not emit them, so no real proof has exercised them. Their semantics may
change.

**Semantics as implemented.** For `x XID X 0 XIDs u CIDs 0`:

1. **Look up the units.** Every id in `CIDs` must name a clause of exactly one
   literal, or the step fails. A clause with more literals gives
   `clause at index not unit`.
2. **Add.** Let `S` be `X` plus the XORs named by `XIDs`. The claimed XOR `X`
   is part of the sum.
3. **Substitute** each unit literal into `S`, starting from the last `CID`. For
   a unit `v` (so `v` is true), a `v` in `S` is removed and the parity flips.
   For a unit `¬v` (so `v` is false), a `v` in `S` is removed.
4. **Accept** when the result is the empty XOR `0 = 0`.

In words: `X` and the sum of the hint XORs must agree once the unit-fixed
variables are substituted into both. So `X` may still mention a unit-fixed
variable; the checker does not require `X` to equal the propagated sum. With the
formula [`example.xnf`](example.xnf), whose clause 3 is the unit `¬x3`:

```
o x 1 1 2 -3 0          XOR 1:  x1 ⊕ x2 ⊕ ¬x3
i x 2 1 2 0 1 2 0       XOR 2:  x1 ⊕ x2
x 3 1 2 3 0 2 u 3 0     accepted: x1 ⊕ x2 ⊕ x3, from XOR 2 and ¬x3
x 4 1 -2 0 1 u 3 0      accepted: x1 ⊕ ¬x2, from XOR 1 and ¬x3
```

The first addition is accepted even though propagating `¬x3` in XOR 2 gives
`x1 ⊕ x2`, not the claimed `x1 ⊕ x2 ⊕ x3`. Both claims follow from the
database, since `¬x3` is in it. Without the `u 3` hint, the first addition
fails.

**For producers.** Write `X` as the propagated sum, without the unit-fixed
variables. That form is accepted now, and it stays valid if the checker is later
narrowed to require `X` to equal the propagated sum.

## 5. Malformed input

| input | result |
|---|---|
| unknown tag or kind byte | parse failure at that record (`failed to parse binary XLRUP record`) |
| record id `0` or odd | parse failure at that record |
| the file ends inside a record | an unterminated last list is read up to the end of the file, and a missing hint list fails the step (`missing … hints`) |
| the number `1` (literal `−0`) in a literal list | ends the list there, and the bytes up to that list's `00` are ignored. The shorter clause or XOR is what gets checked, which is sound but usually fails. |
| an odd number in a hint list (an LRAT-style RAT hint) | ends that hint list, and the step is checked with the hints before it |
| an odd number in a deletion list | ends the deletion list, so fewer ids are deleted |
| a hint naming no clause or XOR | the step fails |

A text proof read without `--no-binary` fails at its first record. In binary
mode, the line number in an error message (`c Checking failed at line: N`) is
the 1-based record number.

## 6. Example

[`example.xlrup`](example.xlrup) and [`example.xlrupb`](example.xlrupb) hold the
same proof, in text and in 38 bytes of binary:

```
text                     binary
o x 1 1 2 -3 0           78 6f  02 02 04 07 00
                         x  o   id 1, lits x1 x2 ¬x3, end
i x 2 1 2 0 1 2 0        78 69  04 02 04 00  02 04 00
                         x  i   id 2, lits x1 x2, end; CIDs 1 2, end
x 3 3 0 1 2 0            78 61  06 06 00  02 04 00  00
                         x  a   id 3, lit x3, end; XIDs 1 2, end; no units, end
i cx 4 3 0 3 0           78 63  08 06 00  06 00
                         x  c   id 4, lit x3, end; XID 3, end
5 0 3 4 0                61  0a 00  06 08 00
                         a   id 5, no lits, end; CIDs 3 4, end
```

`./cake_xlrup example.xnf example.xlrupb` prints `s VERIFIED UNSAT`, and so does
`./cake_xlrup --no-binary example.xnf example.xlrup`.
