"""Compare complete native and Python report construction, including JSON serialization.

Run with both sbml4humans and the built sbml4humans_core wheel installed.
Usage: python scripts/benchmark.py --repeat 3 --output /tmp/sbml-core-benchmark.json
"""
import argparse
import gzip
import json
import logging
import platform
import statistics
import time
from pathlib import Path

import libsbml
import sbmlode
import sbml4humans_core
from sbml4humans.report import report_for_sbml

ROOT = Path(__file__).resolve().parent.parent


def python_report(source: bytes) -> bytes:
    """Build and serialize the existing application's standalone report."""
    return report_for_sbml(source.decode()).report.model_dump_json(by_alias=True).encode()


def measure(function, source: bytes, repeat: int) -> tuple[float, bytes]:
    """Return median wall time and last serialized output after one warm-up."""
    function(source)
    times = []
    for _ in range(repeat):
        start = time.perf_counter()
        output = function(source)
        times.append(time.perf_counter() - start)
    return statistics.median(times), output


def main() -> None:
    """Benchmark representative inputs and verify equality outside timed regions."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repeat', type=int, default=3)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if args.repeat < 1:
        parser.error('--repeat must be positive')
    logging.disable(logging.CRITICAL)
    results = []
    for label, path in [('minimal', 'testdata/minimal_model.xml'),
                        ('e_coli_core', 'testdata/corpus/065.xml.gz'),
                        ('Recon3D', 'testdata/corpus/064.xml.gz')]:
        source = (ROOT / path).read_bytes()
        if path.endswith('.gz'):
            source = gzip.decompress(source)
        native_time, native = measure(sbml4humans_core.report_json, source, args.repeat)
        python_time, reference = measure(python_report, source, args.repeat)
        if json.loads(native) != json.loads(reference):
            raise AssertionError(f'{label}: complete report differs')
        result = {'model': label, 'xml_bytes': len(source), 'json_bytes': len(native),
                  'native_seconds': native_time, 'python_seconds': python_time,
                  'speedup': python_time / native_time}
        results.append(result)
        print(json.dumps(result), flush=True)
    document = {'platform': platform.platform(), 'python': platform.python_version(),
                'core': sbml4humans_core.__version__, 'libsbml': libsbml.getLibSBMLDottedVersion(),
                'sbmlode': sbmlode.__version__, 'repeat': args.repeat, 'results': results}
    if args.output:
        args.output.write_text(json.dumps(document, indent=2) + '\n')


if __name__ == '__main__':
    main()
