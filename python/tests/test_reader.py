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




class NativeSourceTests(unittest.TestCase):
    """The installed wheel must support the entire application source envelope."""

    def test_complete_archives(self) -> None:
        """Compare all source fields, including cross-entry links and equations."""
        target = ROOT / 'testdata/sources'
        manifest = json.loads((target / 'MANIFEST.json').read_text())
        for case in manifest['files']:
            with self.subTest(origin=case['origin']):
                expected = json.loads(gzip.decompress((target / case['report']).read_bytes()))
                options = {'uid': ''}
                if 'budget' in case:
                    options['max_expanded_elements'] = case['budget']
                actual = json.loads(core.source_json((target / case['source']).read_bytes(), **options))
                for entry in actual['reports'].values():
                    entry['debug']['jsonReportTime'] = '0.0 [s]'
                self.assertEqual(actual, expected)

    def test_source_limits_and_recovery(self) -> None:
        """Per-request budgets do not leak to a later request."""
        source = (ROOT / 'testdata/comp_internal.xml').read_bytes()
        with self.assertRaises(ValueError):
            core.source_json(source, limits={'maxContentSize': 1})
        response = json.loads(core.source_json(source))
        self.assertRegex(response['uid'], r'^[a-f0-9]{32}$')
        self.assertIsNotNone(response['reports']['./model.xml']['report']['odeSystem'])
        self.assertEqual(core.SOURCE_CONTRACT, 'source/1')

if __name__ == '__main__':
    unittest.main()
