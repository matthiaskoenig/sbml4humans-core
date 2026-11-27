"""Generate independent unit-factor renderings from the Python report implementation.

Run with the sbml4humans backend environment. The registry version is recorded so
that changes to its display policy remain an explicit parity decision.
"""
import importlib.metadata
import json
from pathlib import Path

from sbml4humans.units import _factor_of

ROOT = Path(__file__).resolve().parent.parent
KINDS = (
    "ampere", "avogadro", "becquerel", "candela", "coulomb", "dimensionless",
    "farad", "gram", "gray", "henry", "hertz", "item", "joule", "katal",
    "kelvin", "kilogram", "liter", "litre", "lumen", "lux", "meter", "metre",
    "mole", "newton", "ohm", "pascal", "radian", "second", "siemens", "sievert",
    "steradian", "tesla", "volt", "watt", "weber",
)

def main() -> None:
    """Write the rendering of each factor in a representative numeric matrix."""
    cases = []
    for kind in KINDS:
        for exponent in (0.0, 0.5, 1.0, 2.0, -1.0, -2.0):
            for scale in (-30, -24, -6, -3, 0, 3, 24, 30, 400, -400):
                for multiplier in (1.0, 2.5, 60.0):
                    result = _factor_of(kind, exponent, scale, multiplier)
                    cases.append({"kind": kind, "exponent": exponent, "scale": scale,
                                  "multiplier": multiplier, "rendered": None if result is None else result[0]})
    output = {"pint": importlib.metadata.version("pint"), "cases": cases}
    (ROOT / "testdata/oracles/units.json").write_text(json.dumps(output, ensure_ascii=False, indent=2)+"\n")

if __name__ == "__main__":
    main()
