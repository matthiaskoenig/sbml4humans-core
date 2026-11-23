"""Harvest the maths of the model corpus of sbml4humans as the fixture of the math gate.

Every math of every function definition, rule, initial assignment, constraint, kinetic law,
trigger, delay, priority and event assignment of the SBML files of the python package is
written as `mathml` (exactly the string `sbml4humans.mathml.astnode_to_latex` feeds its latex
pipeline), `formula` (`formulaToL3String`) and `latex` (`astnode_to_latex`), deduplicated by
`mathml`, to `testdata/oracles/maths.json` together with its own provenance.

Usage, from the backend/ directory of the python repository:
    uv run --frozen python -I /path/to/sbml4humans-core/scripts/math_fixtures.py
"""

import json
import sys
import zipfile
from pathlib import Path

import libsbml
import sbml4humans
from sbml4humans.mathml import astnode_to_latex

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))
import oracle_fixtures  # noqa: E402

RESOURCES = Path(sbml4humans.__file__).resolve().parent / "resources"
CORPUS = ["resources/biomodels/*.omex", "resources/models/**/*.xml", "resources/examples/*.xml"]
MATH_TYPES = {
    "FunctionDefinition",
    "AssignmentRule",
    "RateRule",
    "AlgebraicRule",
    "InitialAssignment",
    "Constraint",
    "KineticLaw",
    "Trigger",
    "Delay",
    "Priority",
    "EventAssignment",
}


def documents() -> list[libsbml.SBMLDocument]:
    """Return the documents of the corpus in the order of the corpus directories."""
    docs = []
    for path in sorted((RESOURCES / "biomodels").glob("*.omex")):
        with zipfile.ZipFile(path) as archive:
            for name in sorted(archive.namelist()):
                if name.lower().endswith(".xml"):
                    docs.append(libsbml.readSBMLFromString(archive.read(name).decode("utf-8")))
    for path in sorted((RESOURCES / "models").glob("**/*.xml")):
        docs.append(libsbml.readSBMLFromFile(str(path)))
    for path in sorted((RESOURCES / "examples").glob("*.xml")):
        docs.append(libsbml.readSBMLFromFile(str(path)))
    return docs


def nodes(doc: libsbml.SBMLDocument) -> list[libsbml.ASTNode]:
    """Return the math of every element of the document which carries one."""
    found = []
    elements = doc.getListOfAllElements()
    for i in range(elements.getSize()):
        element = elements.get(i)
        if type(element).__name__ in MATH_TYPES and element.isSetMath():
            found.append(element.getMath())
    return found


def main() -> None:
    """Write the fixture."""
    entries: dict[str, dict[str, str]] = {}
    total = 0
    for doc in documents():
        for node in nodes(doc):
            total += 1
            mathml = libsbml.writeMathMLToString(node).replace(
                '<?xml version="1.0" encoding="UTF-8"?>', ""
            )
            if mathml not in entries:
                entries[mathml] = {
                    "mathml": mathml,
                    "formula": libsbml.formulaToL3String(node),
                    "latex": astnode_to_latex(node),
                }
    maths = [entries[key] for key in sorted(entries)]
    provenance = {
        "sbml4humans_commit": oracle_fixtures.checkout_commit(),
        "python_libsbml": oracle_fixtures.python_libsbml(),
        "rust_libsbml": oracle_fixtures.rust_libsbml(),
        "corpus": CORPUS,
        "enumeration": (
            "document wide (getListOfAllElements), not the lists of the main model only: "
            "includes the math inside comp model definitions, as the report does"
        ),
        "element_types": sorted(MATH_TYPES),
        "count": total,
        "distinct": len(maths),
    }
    target = ROOT / "testdata" / "oracles" / "maths.json"
    target.write_text(
        json.dumps({"provenance": provenance, "maths": maths}, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(f"{total} maths, {len(maths)} distinct")


if __name__ == "__main__":
    main()
