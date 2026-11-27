"""Generate reader-independent unit inference cases from libSBML and the Python renderer."""
import json
from pathlib import Path

import libsbml
from sbml4humans.units import udef_to_string

ROOT = Path(__file__).resolve().parent.parent
FORMULAS = (
    "a", "b", "c", "d", "1", "pi", "exponentiale", "avogadro", "time",
    "a+b", "a+c", "c+a", "1+a", "a+1", "a*c+b", "c*a+b", "a+b+c",
    "a*a", "a/a", "a/b", "c/a", "a*c", "a*e", "e*e", "a/e", "a*h",
    "a^0", "a^2", "a^c", "a^d", "a^(1+1)", "a^b", "sqrt(a)", "root(3,a)",
    "sqrt(a*e)", "abs(a)", "floor(a)", "ceil(a)", "sin(a)", "log(a)",
    "a+b>c", "piecewise(a,c>0,b)", "delay(a,b)", "max(a,b)", "min(a,c)",
    "quotient(a,b)", "rem(a,b)", "f(a,b)", "g(a)", "unknown(a)", "S", "N",
)

def document(level: int, version: int) -> libsbml.SBMLDocument:
    """A document with distinct global, local, default and named units."""
    doc = libsbml.SBMLDocument(level, version)
    model = doc.createModel(); model.setId("inference")
    if level == 3:
        model.setSubstanceUnits("mole"); model.setTimeUnits("second"); model.setVolumeUnits("litre")
    for sid, scale, multiplier in (("u",-3,1.0),("v",0,2.0)):
        definition=model.createUnitDefinition(); definition.setId(sid)
        unit=definition.createUnit(); unit.setKind(libsbml.UNIT_KIND_MOLE)
        unit.setScale(scale); unit.setMultiplier(multiplier); unit.setExponent(1.0)
    compartment=model.createCompartment(); compartment.setId("volume")
    compartment.setSpatialDimensions(3); compartment.setSize(1); compartment.setConstant(True)
    for sid, amount in (("S",False),("N",True)):
        species=model.createSpecies(); species.setId(sid); species.setCompartment("volume")
        species.setHasOnlySubstanceUnits(amount); species.setBoundaryCondition(False); species.setConstant(False)
    for sid, units, value in (("a","mole",1.0),("b","second",2.0),("c",None,3.0),
                              ("d","dimensionless",2.0),("e","u",4.0),("h","v",5.0),("x",None,0.0)):
        parameter=model.createParameter(); parameter.setId(sid); parameter.setValue(value); parameter.setConstant(sid!="x")
        if units is not None: parameter.setUnits(units)
    for sid, formula in (("f","lambda(x,y,x+y)"),("g","lambda(z,z*f(1,z))")):
        function=model.createFunctionDefinition(); function.setId(sid); function.setMath(libsbml.parseL3Formula(formula))
    return doc


def main() -> None:
    """Write the real unit results and serialized documents for each context."""
    cases=[]
    for level,version in ((2,4),(3,1),(3,2)):
        for context in ("rule","kineticLaw"):
            for formula in FORMULAS:
                doc=document(level,version); model=doc.getModel()
                ast=libsbml.parseL3Formula(formula)
                if ast is None: raise ValueError(formula)
                if context=="rule":
                    owner=model.createAssignmentRule(); owner.setVariable("x")
                else:
                    reaction=model.createReaction(); reaction.setId("r"); reaction.setReversible(False)
                    owner=reaction.createKineticLaw()
                    parameter=owner.createLocalParameter() if level==3 else owner.createParameter()
                    parameter.setId("a"); parameter.setUnits("second"); parameter.setValue(7.0)
                owner.setMath(ast)
                rendered=udef_to_string(owner.getDerivedUnitDefinition())
                cases.append({"level":level,"version":version,"context":context,"formula":formula,
                              "xml":libsbml.writeSBMLToString(doc),"rendered":rendered})
    (ROOT/"testdata/oracles/unit_inference.json").write_text(json.dumps({"libsbml":libsbml.getLibSBMLDottedVersion(),"cases":cases},indent=2)+"\n")

if __name__=="__main__": main()
