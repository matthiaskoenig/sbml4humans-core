# Native report benchmarks

`2026-11-27.json` records a local Linux x86-64 run with CPython 3.14.10, libSBML 5.21.2 and sbmlode 0.3.3. Each measurement includes parsing, the complete report, equation analysis and JSON serialization. Inputs are decompressed before timing. Times are the median of three warm runs after one warm-up. The complete output is checked against Python outside the timed region.

| Model | Python | Native Rust | Speedup |
| --- | ---: | ---: | ---: |
| Minimal | 2.79 ms | 0.21 ms | 13.3x |
| E. coli core | 135 ms | 21.6 ms | 6.2x |
| Recon3D | 17.68 s | 1.51 s | 11.7x |

The ABI3 extension was built with `maturin build --release --locked`. These results measure the native core on one machine, not browser startup, network transfer or UI rendering. Recon3D produces about 115 MB of report JSON, so host deserialization and rendering remain material costs.

Reproduce with both the built wheel and the application's backend installed:

```bash
python scripts/benchmark.py --repeat 3 --output /tmp/sbml-core-benchmark.json
```
