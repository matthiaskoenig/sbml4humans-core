"""Generate the oracle fixtures of the Rust report from the python report of sbml4humans.

The report of every file named on the command line is built by the python implementation
and written to `testdata/oracles/<name>.json`, serialized exactly as the http api sends it.
`testdata/oracles/MANIFEST.json` records which checkout produced them and which libsbml each
side of the gate carries: the one of this interpreter, and the one the Rust crate links. The
report lists the packages the library knows, so the two package sets have to agree for the
gate to mean anything.

Usage: LIBSBML_DIR=/path/to/prefix python scripts/oracle_fixtures.py testdata/minimal_model.xml
"""

import json
import os
import re
import subprocess
import sys
from pathlib import Path

import libsbml
import sbml4humans
from sbml4humans.report import report_for_sbml

ROOT = Path(__file__).resolve().parent.parent
ORACLES = ROOT / "testdata" / "oracles"


def checkout_commit() -> str:
    """Return the commit of the sbml4humans checkout which was imported."""
    root = Path(sbml4humans.__file__).resolve().parent.parent.parent
    result = subprocess.run(
        ["git", "-C", str(root), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def read_header(path: Path) -> str:
    """Return the text of a generated header of the prefix, or say which one is missing."""
    if not path.is_file():
        raise FileNotFoundError(
            f"{path} is missing, set LIBSBML_DIR to the prefix the Rust crate links"
        )
    return path.read_text(encoding="utf-8")


def python_libsbml() -> dict:
    """Return the version and the registered packages of the libsbml of this interpreter."""
    registry = libsbml.SBMLExtensionRegistry
    count = registry.getNumRegisteredPackages()
    return {
        "version": libsbml.getLibSBMLDottedVersion(),
        "packages": sorted(registry.getRegisteredPackageName(i) for i in range(count)),
    }


def rust_libsbml() -> dict:
    """Return the version and the packages of the libsbml prefix the Rust crate links.

    The prefix is the one `crates/libsbml/build.rs` reads, named by `LIBSBML_DIR` or else
    `third_party/prefix`. Both facts come from the headers the build of libsbml generated
    into the prefix: the version from `libsbml-version.h`, the packages from
    `libsbml-config-packages.h`, which holds one `LIBSBML_HAS_PACKAGE_<NAME>` per package
    the build enabled.
    """
    prefix = Path(os.environ.get("LIBSBML_DIR") or ROOT / "third_party" / "prefix")
    common = prefix / "include" / "sbml" / "common"
    version = read_header(common / "libsbml-version.h")
    packages = read_header(common / "libsbml-config-packages.h")
    found = re.findall(r'#define\s+LIBSBML_DOTTED_VERSION\s+"([^"]+)"', version)
    if not found:
        raise ValueError(f"{common / 'libsbml-version.h'} states no version")
    enabled = re.findall(r"#define\s+LIBSBML_HAS_PACKAGE_(\w+)\s+1", packages)
    return {"version": found[0], "packages": sorted(name.lower() for name in enabled)}


def report_json(path: Path) -> dict:
    """Return the report of the one SBML entry of a file, serialized as the api sends it.

    The serializer is `model_dump_json`, the one the http api uses, and not
    `model_dump(mode="json")`: the `ser_json_inf_nan="strings"` of the report model
    (`model.py:32-34`) applies to the JSON serializer of pydantic alone, so the latter
    hands back the float `inf` and `json.dumps` writes it as the bare token `Infinity`,
    which is no JSON at all and the opposite of the contract the api answers,
    `"Infinity"`, `"-Infinity"` and `"NaN"` as strings. No fixture holds a non-finite
    double today and both serializations agree on all four documents of the gate, so
    this changes no committed byte; it becomes the difference between a fixture and a
    file `serde_json` refuses as soon as a document of fbc, whose flux bounds are valued
    `INF` and `-INF`, is gated.
    """
    entry = report_for_sbml(path.read_text(encoding="utf-8"))
    return json.loads(entry.model_dump_json(by_alias=True))["report"]


def main(argv: list[str]) -> None:
    """Write the fixture of every file in argv and the manifest."""
    ORACLES.mkdir(parents=True, exist_ok=True)
    files = []
    for arg in argv:
        path = Path(arg).resolve()
        target = f"{path.stem}.json"
        content = json.dumps(report_json(path), indent=2, sort_keys=True, ensure_ascii=False)
        (ORACLES / target).write_text(content + "\n", encoding="utf-8")
        files.append({"source": path.name, "fixture": target})
    manifest = {
        "sbml4humans_commit": checkout_commit(),
        "python_libsbml": python_libsbml(),
        "rust_libsbml": rust_libsbml(),
        "files": files,
    }
    (ORACLES / "MANIFEST.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main(sys.argv[1:])
