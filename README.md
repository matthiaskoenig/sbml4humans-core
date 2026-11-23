# sbml4humans-core

A Rust workspace which reads SBML documents and builds the report of [sbml4humans](https://sbml4humans.de). It is step 1 of porting the report from the python implementation: an owned document model, a temporary filler which fills it through libsbml, the serde types of the report, the builder, and an oracle which gates the result against the report the python implementation produces.

## Crates

The crates live in `crates/`, each named `sbml4humans-<name>`.

| Crate | Role |
| --- | --- |
| `abi` | The wire contract constant of the report. Part of the port. |
| `model` | The owned document model. Part of the port. |
| `report` | The serde types of the report and the builder which makes a report of a document. Part of the port. |
| `libsbml` | TEMPORARY scaffolding. A C++ shim over libsbml which fills the document model. It is deleted once our own reader in pure Rust agrees with it. |
| `cli` | The binary `sbml4humans-core`, which writes the report of a file as JSON. |
| `oracles` | The gate. It compares the report built here with the report of the python implementation. A member of the workspace but not a default member. |

## Building libsbml for the filler

`crates/libsbml` links libsbml, so the workspace does not build without a libsbml prefix (a directory with `include/` and `lib/`). `crates/libsbml/build.rs` reads the prefix from the environment variable `LIBSBML_DIR`, or else from `third_party/prefix`. Without one of them every command which builds `libsbml` fails at build time: the workspace wide commands, and `-p` of any crate which depends on it (`libsbml`, `cli`, `oracles`). `abi` and `model` do not link libsbml, so `cargo test -p sbml4humans-abi` builds without a prefix. Point `LIBSBML_DIR` at an existing prefix:

```bash
export LIBSBML_DIR=/path/to/prefix
```

`scripts/build-libsbml.sh` builds a static libsbml 5.21.2 into `third_party/prefix`, with exactly the packages python-libsbml 5.21.2 registers, because the report lists the packages the library knows and the oracle gate compares that list. It was written for continuous integration and other machines and has never been run: this slice used an existing prefix instead, so treat the script as unverified.

libsbml is licensed under the LGPL 2.1, so a binary which links `crates/libsbml` carries the conditions of the LGPL 2.1 next to the MIT license of this workspace. Nothing of this workspace is distributed yet, and the constraint goes with the crate once the reader in pure Rust replaces it.

## Usage

```bash
cargo run -p sbml4humans-cli -- report model.xml
cargo run -p sbml4humans-cli -- report --pretty model.xml
```

`report <path>` writes the JSON of the report to stdout. A file which cannot be read or is no SBML document produces a message on stderr and exit code 1, and for `report` the exit code is 0 only when a report was written. `--help` prints the usage.

## Running the gate

The oracle crate is a member but not a default member, so a plain `cargo test` does not run it and `cargo test --workspace` does.

```bash
cargo test                # the default members: abi, cli, libsbml, model, report
cargo test --workspace    # everything, including the gate of the oracles crate
```

Running the gate needs no python at all, because the fixtures of the python report are committed in `testdata/oracles/`. Regenerating them needs a checkout of sbml4humans and `uv`, and the prefix, whose version and package set the manifest records next to the python ones. The fixture of every document named in one run is written by it, together with the manifest, so all of them are named in one run:

```bash
LIBSBML_DIR=/path/to/prefix uv run --frozen --project /path/to/sbml4humans/backend python scripts/oracle_fixtures.py testdata/minimal_model.xml testdata/level2_model.xml testdata/core_elements.xml testdata/level2_reactions.xml
```

`testdata/oracles/maths.json`, the fixture of the math gate, is written by a second generator, which harvests the maths of the model corpus of the python package rather than of a document of `testdata/`:

```bash
LIBSBML_DIR=/path/to/prefix uv run --frozen --project /path/to/sbml4humans/backend python -I scripts/math_fixtures.py
```

The one command which runs everything is the workspace test command. Clippy and the format check are the other two gates, run as below:

```bash
LIBSBML_DIR=/path/to/prefix cargo test --workspace
LIBSBML_DIR=/path/to/prefix cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Licensed under the MIT license, see `LICENSE`.
