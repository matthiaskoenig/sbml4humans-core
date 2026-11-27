"""Native-only host paths preserve report semantics without Python engines."""

import gzip
import json
import unittest
from pathlib import Path

import sbml4humans_core as core

ROOT = Path(__file__).resolve().parents[2]


class HostTests(unittest.TestCase):
    """Exercise the actual installed ABI3 extension's optimized host boundary."""

    def test_report_transport_is_compressed_once_and_diagnostics_are_separate(self):
        path = ROOT / "testdata" / "minimal_model_comp.xml"
        plain, diagnostics = core.report_path(str(path), uid="test")
        compressed, compressed_diagnostics = core.report_path(
            str(path), uid="test", gzip=True
        )
        left = json.loads(plain)
        right = json.loads(gzip.decompress(compressed))
        for response in [left, right]:
            for entry in response["reports"].values():
                entry["debug"]["jsonReportTime"] = ""
        self.assertEqual(left, right)
        self.assertEqual(diagnostics, compressed_diagnostics)
        self.assertTrue(diagnostics)
        self.assertLess(len(compressed), len(plain))

    def test_example_metadata_uses_the_shared_reader_without_reports(self):
        path = ROOT / "testdata" / "validation.xml"
        result = json.loads(core.example_json(str(path), "sbml", None))
        self.assertEqual(result["id"], "validation (validation.xml)")
        self.assertEqual(result["name"], "model with validation errors and warnings")
        self.assertIn("Example model", result["description"])
        self.assertNotIn("report", result)

    @unittest.skipUnless(hasattr(core, "rule_json"), "requires native validation")
    def test_every_documented_rule_matches_the_previous_independent_reader(self):
        expected = json.loads((ROOT / "testdata/oracles/host-rules.json").read_text())
        self.assertGreater(len(expected), 100)
        for rule in expected:
            with self.subTest(code=rule["id"]):
                self.assertEqual(json.loads(core.rule_json(rule["id"])), rule)
                self.assertEqual(core.rule_package(rule["id"]), rule["package"])
        with self.assertRaises(ValueError):
            core.rule_json(42)
