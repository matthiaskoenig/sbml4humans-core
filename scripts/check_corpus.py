"""Compare an installed Python report with the pure Rust CLI on SBML files.

Usage: python scripts/check_corpus.py path/to/models --output /tmp/parity
The output directory retains failed inputs and both reports for diagnosis.
"""
import argparse
import gzip
import json
import logging
import subprocess
from pathlib import Path
from sbml4humans.report import report_for_sbml

MISSING = object()

def differences(ours, theirs, path=''):
    if isinstance(ours, dict) and isinstance(theirs, dict):
        for key in sorted(ours.keys() | theirs.keys()):
            yield from differences(ours.get(key, MISSING), theirs.get(key, MISSING), f'{path}/{key}')
    elif isinstance(ours, list) and isinstance(theirs, list):
        for index in range(max(len(ours), len(theirs))):
            yield from differences(ours[index] if index < len(ours) else MISSING,
                                   theirs[index] if index < len(theirs) else MISSING, f'{path}/{index}')
    elif ours != theirs:
        yield path, ours, theirs

def excerpt(value):
    return '<absent>' if value is MISSING else str(value)[:160]

def main():
    args = argparse.ArgumentParser(description=__doc__)
    args.add_argument('source', type=Path)
    args.add_argument('--output', type=Path, required=True)
    args.add_argument('--cli', type=Path, default=Path(__file__).resolve().parent.parent / 'target/release/sbml4humans-core')
    options = args.parse_args()
    options.output.mkdir(parents=True, exist_ok=True)
    logging.disable(logging.CRITICAL)
    files = sorted(p for p in options.source.rglob('*') if p.is_file() and p.name.lower().endswith(('.xml', '.sbml', '.xml.gz')))
    results = []
    for index, path in enumerate(files):
        label = path.relative_to(options.source).as_posix()
        key = f'{index:04d}_{path.name.removesuffix(".gz")}'
        raw = path.read_bytes()
        if path.suffix == '.gz':
            raw = gzip.decompress(raw)
        source = options.output / key
        source.write_bytes(raw)
        try:
            expected = json.loads(report_for_sbml(raw.decode('utf-8')).model_dump_json(by_alias=True))['report']
            process = subprocess.run([str(options.cli), 'report', str(source)], capture_output=True, timeout=120)
            if process.returncode:
                raise ValueError(process.stderr.decode()[:1000])
            actual = json.loads(process.stdout)
            diff = list(differences(actual, expected))
            if diff:
                (options.output / f'{key}.ours.json').write_text(json.dumps(actual, ensure_ascii=False))
                (options.output / f'{key}.python.json').write_text(json.dumps(expected, ensure_ascii=False))
            else:
                source.unlink()
            result = {'source': label, 'differences': len(diff),
                      'first': [{'path': p, 'ours': excerpt(a), 'python': excerpt(b)} for p, a, b in diff[:6]]}
        except Exception as error:
            result = {'source': label, 'error': str(error)}
        results.append(result)
        print(json.dumps(result, ensure_ascii=False), flush=True)
        (options.output / 'results.json').write_text(json.dumps(results, ensure_ascii=False, indent=2))

if __name__ == '__main__':
    main()
