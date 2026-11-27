"""Generate complete archive-response fixtures from the application, including report fields.

Only request IDs and diagnostic timings are normalized. Report fields are never excluded.
Usage: python scripts/source_fixtures.py /path/to/sbml4humans/backend/sbml4humans/resources
"""
import argparse
import gzip
import json
import logging
import shutil
import sys
import tempfile
from pathlib import Path
from sbml4humans.report import report_for_path
from oracle_fixtures import checkout_commit

ROOT=Path(__file__).resolve().parent.parent

def main():
    """Record each archive's exact manifest, reports, cross-entry links and equations."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('resources',type=Path)
    args=parser.parse_args()
    logging.disable(logging.CRITICAL)
    target=ROOT/'testdata/sources'
    target.mkdir(exist_ok=True)
    files=[]
    paths=sorted((args.resources/'omex').glob('*.omex'))+sorted((args.resources/'biomodels').glob('*.omex'))
    for index,path in enumerate(paths):
        stem=f'{index:03d}'
        response=json.loads(report_for_path(path).model_dump_json(by_alias=True))
        response['uid']=''
        for entry in response['reports'].values(): entry['debug']['jsonReportTime']='0.0 [s]'
        shutil.copyfile(path,target/f'{stem}.omex')
        (target/f'{stem}.json.gz').write_bytes(gzip.compress(json.dumps(response,ensure_ascii=False).encode(),compresslevel=9,mtime=0))
        files.append({'origin':path.relative_to(args.resources).as_posix(),'source':f'{stem}.omex','report':f'{stem}.json.gz'})
        print(index+1,path.name,flush=True)
    # The existing browser/server expansion fixture guards equations at and beyond the exact limit.
    sys.path.insert(0,str(args.resources.parents[1]/'tests'))
    from conftest import _fan_out
    for stem,data,budget in [('fan_out',_fan_out(5),None),('at_budget',_fan_out(2),None)]:
        with tempfile.TemporaryDirectory() as temporary:
            path=Path(temporary)/'model.xml';path.write_bytes(data)
            if stem=='at_budget':
                from sbml4humans.report import _read,_expanded_size,SBML_LOCATION
                with _read(path,False) as read: budget=_expanded_size(read.entries,SBML_LOCATION)
            response=json.loads(report_for_path(path,budget=budget).model_dump_json(by_alias=True))
        response['uid']=''
        for entry in response['reports'].values():entry['debug']['jsonReportTime']='0.0 [s]'
        (target/f'{stem}.xml').write_bytes(data)
        (target/f'{stem}.json.gz').write_bytes(gzip.compress(json.dumps(response,ensure_ascii=False).encode(),compresslevel=9,mtime=0))
        case={'origin':'synthetic/'+stem,'source':f'{stem}.xml','report':f'{stem}.json.gz'}
        if budget is not None:case['budget']=budget
        files.append(case)
    (target/'MANIFEST.json').write_text(json.dumps({'sbml4humans_commit':checkout_commit(),'files':files},indent=2)+'\n')

if __name__=='__main__': main()
