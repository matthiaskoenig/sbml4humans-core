"""Harvest the maths of the model corpus of sbml4humans as the fixture of the math gate.

Every math of every function definition, rule, initial assignment, constraint, kinetic law,
trigger, delay, priority, event assignment, qual function term and distrib uncertainty parameter of the SBML files of the python package is
written as `mathml` (exactly the string `sbml4humans.mathml.astnode_to_latex` feeds its latex
pipeline), `formula` and `latex` (both of `mathml.math_info`, which is what the report writes of a
math: the infix formula of `formulaToL3String` for any node at any depth, and the latex of
`astnode_to_latex` where it can be produced and `null` where it cannot), deduplicated by `mathml`,
to `testdata/oracles/maths.json` together with its own provenance.

The two shapes of `SYNTHETIC` are added to the maths of the corpus, which reaches neither of them.

Every value of an entry is a string or `None`, and a number of a math is a character of the
`formula` or of the `latex` and no double of its own, so the bare `Infinity` which
`json.dumps` writes of a non-finite float cannot arise here the way it can in the report
fixtures of `oracle_fixtures.py`.

Usage, from the backend/ directory of the python repository:
    uv run --frozen python -I /path/to/sbml4humans-core/scripts/math_fixtures.py
"""

import json
import sys
import zipfile
from pathlib import Path

import libsbml
import lxml.etree
import sbml4humans
from sbml4humans.mathml import math_info

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))
import oracle_fixtures  # noqa: E402

RESOURCES = Path(sbml4humans.__file__).resolve().parent / "resources"
CORPUS = [
    "resources/biomodels/*.omex",
    "resources/models/**/*.xml",
    "resources/models/**/*.sbml",
    "resources/examples/*.xml",
]
EXCLUDED = (
    "resources/models/fbc/Recon3D.xml.gz and e_coli_core.xml.gz: the gzipped FBC models "
    "carry no math"
)
# The two shapes which pin the bound of the latex pipeline, each a chain of nested divides
# `1/(1/(...1/x))` of as many divides as the first element of its entry: lxml refuses a mathml
# document deeper than 256 elements, so the math at the bound is typeset and the one past it is
# reported as its formula alone (`mathml.math_info`). No math of the corpus comes anywhere near the
# bound, the deepest of them is 15 elements, so these two are built here and are converted by the
# same python as every other entry of the fixture: the null latex of the second comes from the
# python implementation and not from this script.
SYNTHETIC = {
    "divide_chain_at_the_bound": (
        254,
        "256 mathml elements, the deepest document lxml parses, typeset with its latex",
    ),
    "divide_chain_past_the_bound": (
        255,
        "257 mathml elements, one past the bound, reported without its latex",
    ),
}
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
    "FunctionTerm",
    "UncertParameter",
}


def documents() -> list[libsbml.SBMLDocument]:
    """Return the documents of the corpus in the order of the corpus directories."""
    docs = []
    for path in sorted((RESOURCES / "biomodels").glob("*.omex")):
        with zipfile.ZipFile(path) as archive:
            for name in sorted(archive.namelist()):
                if name.lower().endswith(".xml"):
                    docs.append(libsbml.readSBMLFromString(archive.read(name).decode("utf-8")))
    models = sorted(
        [*(RESOURCES / "models").glob("**/*.xml"), *(RESOURCES / "models").glob("**/*.sbml")]
    )
    for path in models:
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


def synthetic() -> list[libsbml.ASTNode]:
    """The nodes of `SYNTHETIC`, built by the Level 3 parser of libsbml."""
    built = []
    for name, (divides, _) in SYNTHETIC.items():
        node = libsbml.parseL3Formula("1/(" * divides + "x" + ")" * divides)
        if node is None:
            raise ValueError(f"{name} does not parse: {libsbml.getLastParseL3Error()}")
        built.append(node)
    return built


def add(entries: dict[str, dict[str, str | None]], node: libsbml.ASTNode) -> None:
    """Add the math of a node to the entries, keyed and deduplicated by its mathml."""
    mathml = libsbml.writeMathMLToString(node).replace(
        '<?xml version="1.0" encoding="UTF-8"?>', ""
    )
    math = math_info(node, "the fixture of the math gate")
    entry = {"mathml": mathml, "formula": math.formula, "latex": math.latex}
    if mathml not in entries:
        entries[mathml] = entry
    elif entries[mathml] != entry:
        raise AssertionError(
            f"a repeated mathml disagrees with itself: {mathml!r}\n"
            f"first {entries[mathml]!r}\nrepeat {entry!r}"
        )


def main() -> None:
    """Write the fixture."""
    entries: dict[str, dict[str, str | None]] = {}
    total = 0
    for doc in documents():
        for node in nodes(doc):
            total += 1
            add(entries, node)
    for node in synthetic():
        add(entries, node)
    maths = [entries[key] for key in sorted(entries)]
    provenance = {
        "sbml4humans_commit": oracle_fixtures.checkout_commit(),
        "python_libsbml": oracle_fixtures.python_libsbml(),
        "rust_libsbml": oracle_fixtures.rust_libsbml(),
        # the latex is the output of two stylesheets, so the engine which ran
        # them is part of the provenance: `xpath_number` of the rust writer is a
        # port of libxml2's `xmlXPathFormatNumber`, whose switch to exponent
        # notation and whose integer range have changed across versions, and the
        # `mn` template reads libxml2's own lenient `number()`
        "lxml": ".".join(str(part) for part in lxml.etree.LXML_VERSION),
        "libxml2": ".".join(str(part) for part in lxml.etree.LIBXML_VERSION),
        "corpus": CORPUS,
        "enumeration": (
            "document wide (getListOfAllElements), not the lists of the main model only: "
            "includes the math inside comp model definitions, as the report does"
        ),
        "element_types": sorted(MATH_TYPES),
        "excluded": EXCLUDED,
        # `count` is how many maths the corpus holds, before they are deduplicated by
        # their mathml and without the two of `SYNTHETIC`; `distinct` is how many
        # entries the fixture holds, after the deduplication and with those two. So
        # `distinct` is the number of the entries the gate asserts and `count` is no
        # count of anything in the file.
        "count": total,
        "synthetic": {name: why for name, (_, why) in SYNTHETIC.items()},
        "distinct": len(maths),
    }
    target = ROOT / "testdata" / "oracles" / "maths.json"
    target.write_text(
        json.dumps({"provenance": provenance, "maths": maths}, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(f"{total} maths of the corpus and {len(SYNTHETIC)} synthetic, {len(maths)} distinct")


if __name__ == "__main__":
    main()
