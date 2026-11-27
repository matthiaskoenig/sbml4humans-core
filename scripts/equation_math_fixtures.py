"""Pin sbmlode's equation LaTeX, line breaking and errors on the MathML corpus."""
import json
from pathlib import Path
import libsbml
import sbmlode
from sbmlode.printers import LatexPrinter
from sbmlode.astutil import walk

ROOT = Path(__file__).resolve().parent.parent

def main():
    source = json.loads((ROOT / 'testdata/oracles/maths.json').read_text())
    printer = LatexPrinter()
    cases = []
    for case in source['maths']:
        math = libsbml.readMathMLFromString(case['mathml'])
        symbols = {node.getName(): f'symbol_{node.getName()}' for node in walk(math)
                   if node.getType() in {libsbml.AST_NAME, libsbml.AST_FUNCTION}}
        row = {'mathml': case['mathml'], 'symbols': symbols}
        for mode in ['math', 'condition', 'lines']:
            try:
                row[mode] = {'ok': {'math': printer.print, 'condition': printer.print_condition,
                                    'lines': printer.print_lines}[mode](math, symbols)}
            except (ValueError, NotImplementedError) as error:
                row[mode] = {'error': str(error)}
        cases.append(row)
    target = ROOT / 'testdata/oracles/equation_maths.json'
    target.write_text(json.dumps({'sbmlode': sbmlode.__version__, 'libsbml': libsbml.getLibSBMLDottedVersion(),
                                  'cases': cases}, ensure_ascii=False, indent=2) + '\n')
    print(f'{len(cases)} equation math cases')

if __name__ == '__main__':
    main()
