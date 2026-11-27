# sbml4humans-core

A pure Rust SBML report builder for [SBML4Humans](https://sbml4humans.de), with a native Python extension and import-free WebAssembly. The CLI, Python extension and browser worker use the same byte-to-report pipeline. Portable report builds do not link libSBML. An optional native validation feature links libSBML behind the Rust validation interface; browser WASM remains independent of it. The lossless document model, pure Rust reader and equation generator are shared crates of sbmlsim-core. [Native benchmarks](benchmarks/README.md) measure 6.2x faster complete report construction for E. coli core and 11.7x for Recon3D.

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

`source_path_json(path, trusted=False)` exposes `source-path/1` and reads through the same source pipeline without copying the input through Python. With `trusted=True`, standalone models keep their filename and admit comp companions below the selected directory, including recursive references. Canonical paths and symlinks must stay inside that directory. Archives never gain directory companions.

`ode_path_json(path, format, location=None, trusted=False, simulator=False, source_entry=None)` exposes `ode-source/1`. It selects the explicit entry, first master model or first SBML entry, applies the shared comp expansion budget and calls the sbmlsim-core generator. The JSON bytes contain `filename` and `content` for Python, Diffrax, Julia, R, LaTeX, Markdown or Typst.

`HOST_CONTRACT = "host/1"` adds `report_path(path, trusted=False, entry=None, gzip=False, ...) -> (bytes, list[str])`. With `gzip=True`, JSON serializes directly into the compressor at level 5; equation diagnostics travel separately, so a host never needs to decode the report for logging. `example_json(path, mode="sbml", example_id=None)` reads lightweight example metadata without building reports. Modes `archive` and `biomodel` inspect COMBINE manifests and preserve main-model selection. A selected `entry` admits companions only inside its archive directory, using the same confined traversal as trusted filesystem sources. ODE and validation APIs accept the equivalent `source_entry` argument independently of the entry being exported or validated.

Validation and ODE preparation use the shared canonical key builder's lightweight projection. It retains identities, package objects and expansion counts while skipping rendered math, unit inference and copied XML. Validation-enabled wheels also expose `rule_package(code)` and `rule_json(code)` through native libSBML rule tables. Python does not need its own SBML parser or libSBML bindings. All 458 rules used by the application are checked against independent fixtures.

Build with `SBML4HUMANS_CORE_REVISION=$(git rev-parse HEAD)` to stamp `SOURCE_REVISION`; unversioned development builds report `unversioned`. Hosts can reject binaries that do not match their source pin. `LIBSBML_BUILD_ROOT` selects an external native dependency build/cache directory, and `CMAKE_BUILD_PARALLEL_LEVEL` bounds compilation jobs.

Server builds enable native validation explicitly:

```bash
bash scripts/build-libsbml.sh
uv tool run --from 'maturin[patchelf]>=1.16,<2' maturin build --release --features validation --out dist
```

Validation-enabled wheels also expose `VALIDATION_SOURCE_CONTRACT = "validation-source/1"` and `validation_source(path, trusted=False, max_expanded_elements=10000, limits=None)`. Rust admits the source, derives canonical positions through the report key builder and estimates comp expansion with the report's shared analysis. Python supplies no SBML objects, archive members or position maps. The returned session exposes ordered `locations` and `entry_json(location)`, returning `issues` and `skipped`. Entries beyond the expansion budget retain parser diagnostics with `skipped="expandedSize"`. Native ListOf positions inherit their reported owner's key, preserving diagnostic links on unreported containers.

Such wheels expose `VALIDATION_CONTRACT = "validation/1"` and `ValidationSession`. The constructor accepts a sequence of `(location, admitted_file_path, location_uri, positions_json)` records. Position JSON contains `documentPk` and `elements`, the latter a list of `(line, column, depth, primary_key)` records from the report's canonical key builder. The host admits and keeps the files alive until construction completes. Paths are explicit inputs; the validator never discovers neighbouring files or downloads sources.

`session.issues_json(location, check_consistency=True)` returns ordered diagnostics as JSON bytes. Documents are parsed once and checks run once per entry; `check_consistency=False` reads their current parser log without running consistency checks. Native parsing, validation and serialization release the GIL. The comp resolver sees only admitted documents, and its registry is restored after every check. Hosts retain process isolation, deadlines, memory limits and cancellation; separate calls per entry let a killed worker preserve previously completed results. Native allocation failures raise `MemoryError`; ordinary model issues remain successful diagnostic results.

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

The `libsbml` crate provides native validation and the reference reader used by report oracles. Default report and WASM builds need no libSBML prefix. Validation-enabled server wheels and native oracle tests require it. Build the test prefix or point at an existing libSBML 5.21.2 installation:

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

The default host API accepts at most 100 MiB of input and decompressed source content, 1,000 ZIP members and a compression ratio of 200 above 1 MiB. The shared pymetadata-core reader enforces CRC and size checks; manifest declarations retain authored order and duplicates. XML entity expansion is disabled. Math, package nesting and comp expansion have explicit limits. The byte reader and equation resolver perform no filesystem or network access. The explicit native path adapter admits filesystem companions only for trusted standalone sources. Schema validation is a separate application concern.

## Crates

| Crate | Role |
| --- | --- |
| `model` | Owned SBML data and mathematical AST |
| `reader` | Compatibility re-export of the shared `sbmlsim-reader` |
| `report` | Report objects, units, graphs and equation analysis |
| `core` | Shared plain/gzip byte-to-report pipeline |
| `python` | PyO3 native Python extension |
| `wasm` | Import-free buffer ABI for WebAssembly |
| `cli` | JSON report command |
| `abi` | Wire contract identifier |
| `libsbml` | Optional native validation and the C++ reference reader used by oracles |
| `oracles` | Independent full report and focused parity gates |

The Rust implementation is MIT licensed. The optional native validator and reference reader link LGPL-2.1 libSBML. Its upstream license and copyright notices are included in `python/sbml4humans_core/licenses/` and in wheel license metadata. `scripts/build-libsbml.sh` and the public source provide the reproducible native build; `LIBSBML_DIR` selects a separately built prefix.
