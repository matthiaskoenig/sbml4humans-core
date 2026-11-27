# sbml4humans-core

A pure Rust SBML reader and report builder for [SBML4Humans](https://sbml4humans.de), with a native Python extension and import-free WebAssembly. The CLI, Python extension and browser worker use the same byte-to-report pipeline. Production builds do not link libSBML or run Python. [Native benchmarks](benchmarks/README.md) measure 6.2x faster complete report construction for E. coli core and 11.7x for Recon3D.

The report includes core objects, controlled RDF annotations and model history, unit rendering and inference, comp/fbc/qual/distrib objects, the complete reference graph, mathematical expressions and linked differential equations. The reader supports SBML Levels 1 through 3, Content MathML and Level 1 formulas, common XML encodings, namespace aliases, and plain or gzipped SBML. Each report preserves the application's `report/1` JSON contract.

This implements the parsing core for [SBML4Humans #126](https://github.com/matthiaskoenig/sbml4humans/issues/126). The `source` API accepts standalone or gzipped SBML and complete COMBINE archives. It preserves the application envelope and manifest order, resolves supplied external documents, checks MD5 declarations, builds cross-entry graphs and derives equations with confined comp expansion. Request limits and the expanded-model budget are supplied per call. Validation and ODE file downloads remain separate application operations. No external source is fetched.

## Run the CLI

Install the Rust toolchain named in `rust-toolchain.toml`, then:

```bash
cargo run --release --locked -p sbml4humans-cli -- report testdata/comp_internal.xml
cargo run --release --locked -p sbml4humans-cli -- report --pretty model.xml.gz
cargo run --release --locked -p sbml4humans-cli -- source --pretty models.omex
```

The command writes JSON to stdout. Invalid input produces a message on stderr and exit code 1. An equation analysis failure remains in `odeError`, alongside the rest of the successful report.

## Native Python

Build and install a wheel with Python 3.12 or newer and Maturin:

```bash
uv tool run maturin build --release --locked --out dist
uv venv .venv
uv pip install --python .venv/bin/python dist/*.whl
```

```python
import json
from pathlib import Path
from sbml4humans_core import source_json

response = json.loads(source_json(Path("models.omex").read_bytes()))
```

`source_json(bytes) -> bytes` returns the complete `source/1` envelope; `report_json(bytes)` remains the one-document `report/1` API. Both release the GIL for parsing, analysis and serialization. Calls own their document data and can run concurrently. The ABI3 wheel supports Python 3.12 and later. Invalid XML, gzip or archive input raises `ValueError`; source errors also carry `kind`. Host bindings generate request identities and report total source-construction time in the diagnostic timing field.

## Browser and Node

```bash
rustup target add wasm32-unknown-unknown
cd js
npm ci
npm run build
npm test
```

The build writes `js/runtime/sbml4humans-core.wasm` and verifies that it has no host imports. The module needs neither Emscripten nor a JavaScript SBML parser. Type declarations and the application's generated report schema ship alongside the loader.

```javascript
import { createReader } from "./js/index.mjs";

const reader = await createReader();
try {
  const response = await reader.source(await file.arrayBuffer());
  console.log(response.manifest, response.reports);
} finally {
  reader.dispose();
}
```

`createReader` initializes a dedicated module worker, in browsers and Node. Inputs are copied by default. Pass `{ transfer: true }` to transfer ownership of a complete `ArrayBuffer`. Pass `{ signal }` to cancel: an active request terminates its worker, and queued requests continue in a fresh instance. `dispose()` terminates the worker and rejects outstanding requests. Errors carry `kind` (`malformed`, `refused` or `internal`). `./js/wasm.mjs` exposes a synchronous instance API for hosts that already own a worker.

To run the browser example, serve the repository root after building WASM:

```bash
python3 -m http.server 8080
```

Open <http://localhost:8080/js/example/>. Load the bundled comp example or choose a local SBML file. Reports stay in the browser and can be downloaded as JSON.

## Verification

The complete report is compared field by field against committed Python snapshots for 54 standalone fixtures and 70 corpus models, including Recon3D, with no excluded fields. Both pure Rust and the temporary native oracle are checked. Additional gates cover 6,300 unit factors, 312 inference contexts, 1,834 mathematical expressions, equation formatting, package references, encodings and deep inputs. The complete source response is also compared against 53 real COMBINE archives and two expansion-budget fixtures, preserving every report field. Node and installed native-wheel tests exercise the actual artifacts against every standalone and source fixture. Chromium tests check full report parity, worker responsiveness, cancellation, recovery and the example page.

The temporary `libsbml` crate is oracle scaffolding. Only native oracle and report tests require its prefix; production builds do not. Build the test prefix or point at an existing libSBML 5.21.2 installation:

```bash
bash scripts/build-libsbml.sh
export LIBSBML_DIR="$PWD/third_party/prefix"
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --release --locked
python -m unittest discover -s python/tests -q
cd js
npm test
npx playwright install chromium
npm run test:browser
```

The native binding test must run with the built wheel installed. Regenerating snapshots requires the existing SBML4Humans backend environment; generators in `scripts/` record the source commit and library versions. `scripts/oracle_fixtures.py` regenerates the standalone manifest from all XML paths passed to it; `scripts/corpus_fixtures.py` snapshots the bundled application model corpus. `scripts/sync_contract.py /path/to/sbml4humans --check` verifies that generated frontend types and schema remain synchronized. `scripts/benchmark.py` compares complete native and Python report construction, including JSON serialization, and checks equality outside the timed region.

The default host API accepts at most 100 MiB of input and decompressed source content, 1,000 ZIP members and a compression ratio of 200 above 1 MiB. The shared pymetadata-core reader enforces CRC and size checks; manifest declarations retain authored order and duplicates. XML entity expansion is disabled. Math, package nesting and comp expansion have explicit limits. The reader and equation resolver perform no filesystem or network access. Schema validation is a separate application concern.

## Crates

| Crate | Role |
| --- | --- |
| `model` | Owned SBML data and mathematical AST |
| `reader` | Pure Rust XML, metadata, package and math reader |
| `report` | Report objects, units, graphs and equation analysis |
| `core` | Shared plain/gzip byte-to-report pipeline |
| `python` | PyO3 native Python extension |
| `wasm` | Import-free buffer ABI for WebAssembly |
| `cli` | JSON report command |
| `abi` | Wire contract identifier |
| `libsbml` | Temporary C++ oracle adapter, not a production dependency |
| `oracles` | Independent full report and focused parity gates |

MIT licensed. The temporary native oracle links LGPL-2.1 libSBML; the production parser and bindings do not.
