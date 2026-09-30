# DAG · causal fronts of integration readiness

Revision 1.1: 84 packets, no calendar speed estimates. Tests/contracts may be written before dependencies are implemented; final acceptance requires real producers. A04 is V1 contract-only; the V2 economic engine is not part of the graph.

| Front | Packets |
|---|---|
| 1 | F01 |
| 2 | F02 |
| 3 | F03 |
| 4 | F04, F05 |
| 5 | F06, I01 |
| 6 | I02, O01 |
| 7 | I03, I04, L01, M01, N01, O02, U07 |
| 8 | I05, L02, L03, N02, N06, O03 |
| 9 | D01, I06, L04, N03, N04, N05, O07 |
| 10 | A01, D02, O08, P01 |
| 11 | A02, D03, G01, P02 |
| 12 | D04, G02, P03, Q01 |
| 13 | D05, G03, O04, P04 |
| 14 | D06, G04, O05, U01 |
| 15 | A03, P05 |
| 16 | A04, A05, G05, L05, O06 |
| 17 | A06, G06, L06, M02, M03, P06, Q02, U02 |
| 18 | M05, Q03, U03, X03, X04 |
| 19 | Q04, Q05, U05, X02 |
| 20 | M04, M07, Q06 |
| 21 | M06, U04 |
| 22 | U08 |
| 23 | U06, X05, X07 |
| 24 | X01 |
| 25 | X06 |

Main new chains:

```text
F02/I02 → O02/O03 → O07 → O08 → O06 → U02 → X04
F02/F06/I02/F04 → U07 → U01 → U04 → U08
A02 → Q01 → Q02 → Q03 → Q04/Q05 → Q06
Q04/Q05 → M07 → U08 → X07 → X06
```

This is an overview; the full source of truth is depends_on in backlog.json. U07 does not require the economic engine, O03 does not require BFT/stake transport, Q01 does not require result/settlement. Boundary complexity is covered by separate tests, not by cutting down the product.
