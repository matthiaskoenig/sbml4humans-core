"""Copy the generated application wire contract without changing generated files.

Usage: python3 scripts/sync_contract.py /path/to/sbml4humans [--check]
"""
import argparse
from pathlib import Path


def main() -> None:
    """Copy or verify the contract produced by the application's schema generator."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sbml4humans", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    for source, target in [
        ("frontend/src/types/report.ts", "js/report.d.ts"),
        ("frontend/src/schema/report.schema.json", "js/report.schema.json"),
    ]:
        content = (args.sbml4humans / source).read_bytes()
        path = root / target
        if args.check:
            if path.read_bytes() != content:
                raise SystemExit(f"{target} differs from {source}; regenerate the contract")
        else:
            path.write_bytes(content)


if __name__ == "__main__":
    main()
