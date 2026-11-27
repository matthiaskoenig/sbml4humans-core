"""Exercise optional native validation through the installed ABI3 wheel."""

import json
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


if __name__ == "__main__":
    unittest.main()
