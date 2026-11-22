"""Generate the oracle fixtures of the Rust report from the python report of sbml4humans.

The report of every file named on the command line is built by the python implementation
and written to `testdata/oracles/<name>.json`, serialized exactly as the http api sends it.
`testdata/oracles/MANIFEST.json` records which checkout and which libsbml produced them.

Usage: python scripts/oracle_fixtures.py testdata/minimal_model.xml
"""

import json
import subprocess
import sys
from pathlib import Path

import libsbml
import sbml4humans
from sbml4humans.report import report_for_path

ORACLES = Path(__file__).resolve().parent.parent / "testdata" / "oracles"


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


def report_json(path: Path) -> dict:
    """Return the report of the one SBML entry of a file, serialized by alias."""
    response = report_for_path(path, trusted=True)
    entries = response.reports
    if len(entries) != 1:
        raise ValueError(f"{path} has {len(entries)} entries, expected one")
    entry = next(iter(entries.values()))
    return entry.model_dump(mode="json", by_alias=True)["report"]


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
        "libsbml_version": libsbml.getLibSBMLDottedVersion(),
        "files": files,
    }
    (ORACLES / "MANIFEST.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main(sys.argv[1:])
