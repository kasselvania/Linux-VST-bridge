# WR0A retained live readback

| Fact | Result |
|---|---|
| Classification | `WR0A_LIVE_MATCH` |
| Environment identity | `d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4` |
| Transaction | `wr0-20260901T053317Z-183afd5bbf0736e4` |
| Runner identity | `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547` |
| Contract source | `c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541` |
| Workload | `4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac` |
| Retired replacement record | `917c939c605f457376b0bdd53e06d252d4aecee4b39728850b2e1a61aa102b3e` |
| Archive/live equality | `true` |
| Before/after physical equality | `true` |
| Protected fixture equality | `true` |
| Transaction siblings before/after | `0 / 0` |
| Forbidden process counts before/after | `all zero / all zero` |
| Bytecode artifacts | `0` |
| Source-tree change | `none` |

The retained readback was produced by a fresh renderer-owned session. It did
not consume diagnostic stdout. The dispatcher imported the exact committed
`launch.py` under the fixed non-main module name with bytecode disabled and
called only the seven approved read-only functions.
