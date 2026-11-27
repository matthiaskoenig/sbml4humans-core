"""Exercise optional native validation through the installed ABI3 wheel."""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import sbml4humans_core as core

ROOT = Path(__file__).resolve().parents[2]


@unittest.skipUnless(
    getattr(core, "VALIDATION_CONTRACT", None) == "validation/1",
    "native validation feature is not enabled",
)
class NativeValidationTests(unittest.TestCase):
    """Native documents retain parser diagnostics and are checked only once."""

    def test_complete_diagnostics_and_recovery(self) -> None:
        path = ROOT / "testdata" / "validation.xml"
        positions = json.dumps(
            {
                "documentPk": "document/SBMLDocument:document",
                "elements": [[1, 1, 1, "validation/Model:validation"]],
            }
        )
        session = core.ValidationSession(
            [("./model.xml", str(path), str(path), positions)]
        )
        read = json.loads(session.issues_json("model.xml", check_consistency=False))
        checked = json.loads(session.issues_json("model.xml"))
        self.assertGreater(len(checked), len(read))
        self.assertIn(10601, {issue["rule"] for issue in checked})
        self.assertNotIn(21152, {issue["rule"] for issue in checked})
        self.assertEqual(checked, json.loads(session.issues_json("./model.xml")))
        with self.assertRaises(ValueError):
            session.issues_json("../outside.xml")
        self.assertEqual(checked, json.loads(session.issues_json("model.xml")))

    def test_file_input_preserves_utf16_diagnostics(self) -> None:
        xml = '<?xml version="1.0" encoding="UTF-16"?><sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" level="3" version="2"><model id="m"/></sbml>'
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "model.xml"
            path.write_bytes(xml.encode("utf-16"))
            positions = json.dumps(
                {"documentPk": "document/SBMLDocument:document", "elements": []}
            )
            session = core.ValidationSession(
                [("./model.xml", str(path), str(path), positions)]
            )
            issues = json.loads(session.issues_json("model.xml"))
            self.assertEqual([issue["rule"] for issue in issues], [10101])

    def test_source_preparation_uses_report_keys_and_keeps_limits(self) -> None:
        path = ROOT / "testdata" / "validation.xml"
        session = core.validation_source(str(path))
        self.assertEqual(session.locations, ["./model.xml"])
        result = json.loads(session.entry_json("model.xml"))
        self.assertIsNone(result["skipped"])
        issues = result["issues"]
        self.assertIn(10601, {issue["rule"] for issue in issues})
        self.assertIn("validation/Parameter:k1", {issue["pk"] for issue in issues})
        for operation in (
            core.validation_source,
            lambda path, **options: core.ode_path_json(path, "python", **options),
        ):
            with self.assertRaises(ValueError) as raised:
                operation(str(path), limits={"maxContentSize": 1})
            self.assertEqual(raised.exception.kind, "refused")
        self.assertEqual(result, json.loads(session.entry_json("./model.xml")))

    def test_comp_budget_cannot_be_bypassed_with_a_location_alias(self) -> None:
        session = core.validation_source(
            str(ROOT / "testdata" / "minimal_model_comp.xml"),
            trusted=True,
            max_expanded_elements=1,
        )
        expected = json.loads(session.entry_json(session.locations[0]))
        self.assertEqual(expected["skipped"], "expandedSize")
        self.assertEqual(
            expected,
            json.loads(session.entry_json(session.locations[0].removeprefix("./"))),
        )

    @unittest.skipUnless(sys.platform == "linux", "requires Linux address-space limits")
    def test_native_allocation_failure_is_memory_error(self) -> None:
        """An isolated native allocation failure keeps the host's memory status."""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "large.xml"
            with path.open("w") as stream:
                stream.write(
                    '<?xml version="1.0" encoding="UTF-8"?><sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" level="3" version="2"><model id="m"><listOfParameters>'
                )
                for index in range(100000):
                    stream.write(
                        f'<parameter id="p{index}" value="1" constant="true"/>'
                    )
                stream.write("</listOfParameters></model></sbml>")
            code = """
import json, resource, sys
from pathlib import Path
import sbml4humans_core as core
size = int(next(line.split()[1] for line in Path('/proc/self/status').read_text().splitlines() if line.startswith('VmSize:'))) * 1024
resource.setrlimit(resource.RLIMIT_AS, (size + 32 * 1024 * 1024,) * 2)
positions = json.dumps({'documentPk': 'document/SBMLDocument:document', 'elements': []})
try:
    core.ValidationSession([('./model.xml', sys.argv[1], sys.argv[1], positions)])
except MemoryError:
    print('native-memory-error')
else:
    raise AssertionError('native parsing unexpectedly fit within the memory budget')
"""
            result = subprocess.run(
                [sys.executable, "-c", code, str(path)],
                capture_output=True,
                text=True,
                timeout=30,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.strip(), "native-memory-error")


if __name__ == "__main__":
    unittest.main()
