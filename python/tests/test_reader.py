"""Exercise the installed ABI3 wheel against the complete wire contract."""
import concurrent.futures
import gzip
import json
import unittest
from pathlib import Path

import sbml4humans_core as core

ROOT = Path(__file__).resolve().parents[2]


class NativeReaderTests(unittest.TestCase):
    """Native bindings must preserve report data and recover after failures."""

    def test_complete_reports(self) -> None:
        """Match every oracle through both plain and compressed input."""
        manifest = json.loads((ROOT / 'testdata/oracles/MANIFEST.json').read_text())
        for case in manifest['files']:
            with self.subTest(source=case['source']):
                source = (ROOT / 'testdata' / case['source']).read_bytes()
                expected = json.loads((ROOT / 'testdata/oracles' / case['fixture']).read_bytes())
                for content in (source, gzip.compress(source)):
                    output = core.report_json(content)
                    self.assertIsInstance(output, bytes)
                    self.assertEqual(json.loads(output), expected)

    def test_concurrent_reports_and_recovery(self) -> None:
        """Calls release the GIL and keep per-document state isolated."""
        source = (ROOT / 'testdata/comp_internal.xml').read_bytes()
        expected = core.report_json(source)
        for source_error in (b'not XML', b'\x1f\x8b\x08'):
            with self.assertRaises(ValueError):
                core.report_json(source_error)
        with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
            self.assertEqual(list(executor.map(core.report_json, [source] * 12)), [expected] * 12)
        self.assertEqual(core.REPORT_CONTRACT, 'report/1')


if __name__ == '__main__':
    unittest.main()
