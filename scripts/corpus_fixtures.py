"""Pin standalone report parity on the bundled SBML4Humans model corpus.

Usage: python scripts/corpus_fixtures.py /path/to/sbml4humans/backend/sbml4humans/resources
Compressed sources and full reports are deterministic and require no server at test time.
"""
import argparse
import gzip
import json
import logging
import zipfile
from pathlib import Path
import libsbml
import sbmlode
from sbml4humans.report import report_for_sbml
from oracle_fixtures import checkout_commit

ROOT = Path(__file__).resolve().parent.parent

def inputs(resources):
    for path in sorted((resources / 'biomodels').glob('*.omex')):
        with zipfile.ZipFile(path) as archive:
            for member in sorted(archive.namelist()):
                if member.lower().endswith(('.xml', '.sbml')):
                    source = archive.read(member)
                    if b'<sbml' in source:
                        yield path.relative_to(resources).as_posix() + '#' + member, source
    for path in sorted((resources / 'models').rglob('*')):
        if path.is_file() and path.name.endswith(('.xml', '.sbml', '.xml.gz')):
            source = path.read_bytes()
            yield path.relative_to(resources).as_posix(), gzip.decompress(source) if path.suffix == '.gz' else source

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('resources', type=Path)
    args = parser.parse_args()
    logging.disable(logging.CRITICAL)
    target = ROOT / 'testdata/corpus'
    target.mkdir(exist_ok=True)
    files = []
    for index, (origin, source) in enumerate(inputs(args.resources)):
        stem = f'{index:03d}'
        report = report_for_sbml(source.decode()).report.model_dump_json(by_alias=True).encode()
        (target / f'{stem}.xml.gz').write_bytes(gzip.compress(source, compresslevel=9, mtime=0))
        (target / f'{stem}.json.gz').write_bytes(gzip.compress(report, compresslevel=9, mtime=0))
        files.append({'origin': origin, 'source': f'{stem}.xml.gz', 'report': f'{stem}.json.gz'})
        print(index + 1, origin, flush=True)
    (target / 'MANIFEST.json').write_text(json.dumps({
        'sbml4humans_commit': checkout_commit(), 'libsbml': libsbml.getLibSBMLDottedVersion(),
        'sbmlode': sbmlode.__version__, 'files': files,
    }, indent=2) + '\n')

if __name__ == '__main__':
    main()
