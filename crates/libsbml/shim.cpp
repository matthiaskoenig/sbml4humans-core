/*
 * The C layer over the C++ API of libsbml which sbml4humans-libsbml calls.
 *
 * libsbml ships a C API, but it is declared per class rather than on SBase, so
 * the id, the serialized XML, the plugins of the packages and the error log of
 * an element are not reachable through it. One shim over the C++ API keeps the
 * whole surface the filler uses under one convention instead:
 *
 *  - Every element of a document is an `SBase*`. The functions of one class
 *    cast it down with `dynamic_cast` and answer their empty value for an
 *    element of another class, so a pointer cannot be mistaken for one of a
 *    class it is not and the Rust side needs no cast of its own.
 *  - A function which answers a `const char*` borrows the string of libsbml,
 *    which lives as long as the document. A function which answers a `char*`
 *    hands over the string, which `s4h_string_free` frees.
 *  - A function which answers an optional attribute returns 1 and writes the
 *    value when the document sets the attribute, and 0 without touching the
 *    value when it does not. An attribute which is not set has no value, never
 *    the default of its type.
 *  - An exception must not cross into Rust, which would end the process, so
 *    every function which allocates catches everything and answers its empty
 *    value. The plain getters of a number or of a reference cannot throw.
 */

#include <cstdlib>
#include <cstring>
#include <string>

#include <sbml/SBMLTypes.h>
#include <sbml/SBMLNamespaces.h>
#include <sbml/extension/SBasePlugin.h>
#include <sbml/extension/SBMLExtensionRegistry.h>

LIBSBML_CPP_NAMESPACE_USE

namespace {

/// A copy of a string of libsbml which `s4h_string_free` frees, null on failure.
char* handed_over(const std::string& text) {
    char* copy = static_cast<char*>(std::malloc(text.size() + 1));
    if (copy == nullptr) {
        return nullptr;
    }
    std::memcpy(copy, text.c_str(), text.size() + 1);
    return copy;
}

/// The string of a reference of libsbml, null when the attribute is not set.
///
/// `text` has to come from a getter of libsbml which answers a
/// `const std::string&`, a string of the document which outlives the call. A
/// getter which answers a string by value, `SBase::getNotesString`,
/// `SBase::getAnnotationString`, `SBase::getSBOTermID`, `SBase::getURI` and
/// `SBase::getPrefix` among them, binds a temporary here which dies at the end
/// of the `return` of the caller and would hand Rust a pointer into freed
/// memory; such a getter goes through `handed_over` instead.
///
/// `getPrefix` is the one name which collides. `SBasePlugin::getPrefix`
/// answers a `const std::string&` and is borrowed below; `SBase::getPrefix`
/// answers a `std::string` by value and must never be.
const char* borrowed(bool is_set, const std::string& text) {
    return is_set ? text.c_str() : nullptr;
}

}  // namespace

extern "C" {

/// The package of an SBML Level 3 extension a document declares.
struct S4hPackage {
    /// The prefix of the namespace of the package, null when there is none.
    const char* prefix;
    /// The version of the package.
    unsigned int version;
    /// Whether the document declares the package as required.
    int required;
};

/// Frees a string a function of the shim handed over.
void s4h_string_free(char* text) {
    std::free(text);
}

/* ---------------------------------------------------------------- document */

/// Reads a document of the XML of a nul terminated string, null on failure.
SBMLDocument* s4h_read(const char* xml) {
    try {
        return readSBMLFromString(xml);
    } catch (...) {
        return nullptr;
    }
}

/// Frees a document of `s4h_read`.
void s4h_document_free(SBMLDocument* doc) {
    delete doc;
}

/// The SBML level of a document.
unsigned int s4h_document_level(SBMLDocument* doc) {
    return doc->getLevel();
}

/// The SBML version of a document within its level.
unsigned int s4h_document_version(SBMLDocument* doc) {
    return doc->getVersion();
}

/// The document itself as an element.
SBase* s4h_document_sbase(SBMLDocument* doc) {
    return doc;
}

/// The model of a document, null when it has none.
SBase* s4h_document_model(SBMLDocument* doc) {
    return doc->getModel();
}

/// The errors of the read of a document in the words of libsbml, handed over.
char* s4h_document_error_log(SBMLDocument* doc) {
    try {
        SBMLErrorLog* log = doc->getErrorLog();
        return log == nullptr ? nullptr : handed_over(log->toString());
    } catch (...) {
        return nullptr;
    }
}

/// Whether the read of a document ran out of memory.
int s4h_document_out_of_memory(SBMLDocument* doc) {
    SBMLErrorLog* log = doc->getErrorLog();
    if (log == nullptr) {
        return 0;
    }
    for (unsigned int n = 0; n < log->getNumErrors(); ++n) {
        const SBMLError* error = log->getError(n);
        if (error != nullptr && error->getErrorId() == XMLOutOfMemory) {
            return 1;
        }
    }
    return 0;
}

/// The number of errors and fatal errors libsbml logged of the read of a document.
unsigned int s4h_document_num_errors(SBMLDocument* doc) {
    SBMLErrorLog* log = doc->getErrorLog();
    if (log == nullptr) {
        return 0;
    }
    return log->getNumFailsWithSeverity(LIBSBML_SEV_ERROR) +
           log->getNumFailsWithSeverity(LIBSBML_SEV_FATAL);
}

/// The number of plugins of a document, of which some are packages.
unsigned int s4h_document_num_plugins(SBMLDocument* doc) {
    return doc->getNumPlugins();
}

/// The package of plugin `n` of a document.
///
/// The prefix is null when there is no plugin `n` and when plugin `n` is none
/// of a package: libsbml implements the math of Level 3 Version 2 as a plugin
/// of the document which carries the namespace of core itself.
S4hPackage s4h_document_package(SBMLDocument* doc, unsigned int n) {
    S4hPackage package = {nullptr, 0, 0};
    try {
        SBasePlugin* plugin = doc->getPlugin(n);
        if (plugin == nullptr) {
            return package;
        }
        const std::string& prefix = plugin->getPrefix();
        const std::string core =
            SBMLNamespaces::getSBMLNamespaceURI(doc->getLevel(), doc->getVersion());
        if (plugin->getURI() == core) {
            return package;
        }
        package.prefix = prefix.c_str();
        package.version = plugin->getPackageVersion();
        package.required = doc->getPackageRequired(plugin->getPackageName()) ? 1 : 0;
        return package;
    } catch (...) {
        package.prefix = nullptr;
        return package;
    }
}

/* ----------------------------------------------------------------- element */

/// The `id` of an element, null when it is not set.
///
/// `getId` is the `id` attribute of every class the filler reads. The classes
/// which alias it to another attribute, the initial assignment, the rule and
/// the event assignment, carry theirs in `getIdAttribute` instead and will
/// need a function of their own.
const char* s4h_sbase_id(SBase* sbase) {
    return borrowed(sbase->isSetId(), sbase->getId());
}

/// The `metaid` of an element, null when it is not set.
const char* s4h_sbase_meta_id(SBase* sbase) {
    return borrowed(sbase->isSetMetaId(), sbase->getMetaId());
}

/// The `name` of an element, null when it is not set.
const char* s4h_sbase_name(SBase* sbase) {
    return borrowed(sbase->isSetName(), sbase->getName());
}

/// The id of the SBO term of an element, handed over, null when it is not set.
char* s4h_sbase_sbo_term_id(SBase* sbase) {
    try {
        return sbase->isSetSBOTerm() ? handed_over(sbase->getSBOTermID()) : nullptr;
    } catch (...) {
        return nullptr;
    }
}

/// The `notes` of an element as XML, handed over, null when it has none.
char* s4h_sbase_notes(SBase* sbase) {
    try {
        return sbase->isSetNotes() ? handed_over(sbase->getNotesString()) : nullptr;
    } catch (...) {
        return nullptr;
    }
}

/// The `annotation` of an element as XML, handed over, null when it has none.
char* s4h_sbase_annotation(SBase* sbase) {
    try {
        return sbase->isSetAnnotation() ? handed_over(sbase->getAnnotationString()) : nullptr;
    } catch (...) {
        return nullptr;
    }
}

/// The element serialized back to XML, handed over, null on failure.
char* s4h_sbase_xml(SBase* sbase) {
    try {
        return sbase->toSBML();
    } catch (...) {
        return nullptr;
    }
}

/// The line where an element starts in the source.
unsigned int s4h_sbase_line(SBase* sbase) {
    return sbase->getLine();
}

/// The column where an element starts in the source.
unsigned int s4h_sbase_column(SBase* sbase) {
    return sbase->getColumn();
}

/* ------------------------------------------------------------------- model */

/// The number of compartments of a model.
unsigned int s4h_model_num_compartments(SBase* sbase) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? 0 : model->getNumCompartments();
}

/// Compartment `n` of a model, null when there is none.
SBase* s4h_model_compartment(SBase* sbase, unsigned int n) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? nullptr : model->getCompartment(n);
}

/// The number of species of a model.
unsigned int s4h_model_num_species(SBase* sbase) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? 0 : model->getNumSpecies();
}

/// Species `n` of a model, null when there is none.
SBase* s4h_model_species(SBase* sbase, unsigned int n) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? nullptr : model->getSpecies(n);
}

/// The number of parameters of a model.
unsigned int s4h_model_num_parameters(SBase* sbase) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? 0 : model->getNumParameters();
}

/// Parameter `n` of a model, null when there is none.
SBase* s4h_model_parameter(SBase* sbase, unsigned int n) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? nullptr : model->getParameter(n);
}

/* ------------------------------------------------------------- compartment */

/// The `spatialDimensions` of a compartment.
int s4h_compartment_spatial_dimensions(SBase* sbase, double* value) {
    Compartment* compartment = dynamic_cast<Compartment*>(sbase);
    if (compartment == nullptr || !compartment->isSetSpatialDimensions()) {
        return 0;
    }
    *value = compartment->getSpatialDimensionsAsDouble();
    return 1;
}

/// The `size` of a compartment.
int s4h_compartment_size(SBase* sbase, double* value) {
    Compartment* compartment = dynamic_cast<Compartment*>(sbase);
    if (compartment == nullptr || !compartment->isSetSize()) {
        return 0;
    }
    *value = compartment->getSize();
    return 1;
}

/// The `constant` of a compartment.
int s4h_compartment_constant(SBase* sbase, int* value) {
    Compartment* compartment = dynamic_cast<Compartment*>(sbase);
    if (compartment == nullptr || !compartment->isSetConstant()) {
        return 0;
    }
    *value = compartment->getConstant() ? 1 : 0;
    return 1;
}

/// The `units` of a compartment, null when they are not set.
const char* s4h_compartment_units(SBase* sbase) {
    Compartment* compartment = dynamic_cast<Compartment*>(sbase);
    if (compartment == nullptr) {
        return nullptr;
    }
    return borrowed(compartment->isSetUnits(), compartment->getUnits());
}

/* ----------------------------------------------------------------- species */

/// The `compartment` of a species, null when it is not set.
const char* s4h_species_compartment(SBase* sbase) {
    Species* species = dynamic_cast<Species*>(sbase);
    if (species == nullptr) {
        return nullptr;
    }
    return borrowed(species->isSetCompartment(), species->getCompartment());
}

/// The `initialAmount` of a species.
int s4h_species_initial_amount(SBase* sbase, double* value) {
    Species* species = dynamic_cast<Species*>(sbase);
    if (species == nullptr || !species->isSetInitialAmount()) {
        return 0;
    }
    *value = species->getInitialAmount();
    return 1;
}

/// The `initialConcentration` of a species.
int s4h_species_initial_concentration(SBase* sbase, double* value) {
    Species* species = dynamic_cast<Species*>(sbase);
    if (species == nullptr || !species->isSetInitialConcentration()) {
        return 0;
    }
    *value = species->getInitialConcentration();
    return 1;
}

/// The `substanceUnits` of a species, null when they are not set.
const char* s4h_species_substance_units(SBase* sbase) {
    Species* species = dynamic_cast<Species*>(sbase);
    if (species == nullptr) {
        return nullptr;
    }
    return borrowed(species->isSetSubstanceUnits(), species->getSubstanceUnits());
}

/// The `hasOnlySubstanceUnits` of a species.
int s4h_species_has_only_substance_units(SBase* sbase, int* value) {
    Species* species = dynamic_cast<Species*>(sbase);
    if (species == nullptr || !species->isSetHasOnlySubstanceUnits()) {
        return 0;
    }
    *value = species->getHasOnlySubstanceUnits() ? 1 : 0;
    return 1;
}

/// The `boundaryCondition` of a species.
int s4h_species_boundary_condition(SBase* sbase, int* value) {
    Species* species = dynamic_cast<Species*>(sbase);
    if (species == nullptr || !species->isSetBoundaryCondition()) {
        return 0;
    }
    *value = species->getBoundaryCondition() ? 1 : 0;
    return 1;
}

/// The `constant` of a species.
int s4h_species_constant(SBase* sbase, int* value) {
    Species* species = dynamic_cast<Species*>(sbase);
    if (species == nullptr || !species->isSetConstant()) {
        return 0;
    }
    *value = species->getConstant() ? 1 : 0;
    return 1;
}

/* --------------------------------------------------------------- parameter */

/// The `value` of a parameter.
int s4h_parameter_value(SBase* sbase, double* value) {
    Parameter* parameter = dynamic_cast<Parameter*>(sbase);
    if (parameter == nullptr || !parameter->isSetValue()) {
        return 0;
    }
    *value = parameter->getValue();
    return 1;
}

/// The `constant` of a parameter.
int s4h_parameter_constant(SBase* sbase, int* value) {
    Parameter* parameter = dynamic_cast<Parameter*>(sbase);
    if (parameter == nullptr || !parameter->isSetConstant()) {
        return 0;
    }
    *value = parameter->getConstant() ? 1 : 0;
    return 1;
}

/// The `units` of a parameter, null when they are not set.
const char* s4h_parameter_units(SBase* sbase) {
    Parameter* parameter = dynamic_cast<Parameter*>(sbase);
    if (parameter == nullptr) {
        return nullptr;
    }
    return borrowed(parameter->isSetUnits(), parameter->getUnits());
}

/* ------------------------------------------------------------- syntax tree */

/// Reads the syntax tree of the content MathML of a nul terminated string.
///
/// The caller owns the tree and frees it with `s4h_ast_free`. Null says that
/// the string is no content MathML: `readMathMLFromString` logs the errors of
/// the read on a log of its own, deletes the tree it had built and answers
/// null, so there is nothing to free of a failure and no log to read.
ASTNode* s4h_read_math_from_string(const char* xml) {
    try {
        return readMathMLFromString(xml);
    } catch (...) {
        return nullptr;
    }
}

/// Frees a syntax tree of `s4h_read_math_from_string` with its children.
void s4h_ast_free(ASTNode* node) {
    delete node;
}

/// The kind of a node as the code of the `AstKind` of the report.
///
/// The codes are the contract with `sbml4humans_model::AstKind`, whose name is
/// the comment of every case. A type which is named by no case, the types of
/// the packages and of the MathML of the extended math which the report has no
/// kind of among them, is 0, the kind `Other`, which the writers refuse: the
/// kind of a type the report does not know is never guessed to be a near one.
int s4h_ast_kind(ASTNode* node) {
    switch (node->getType()) {
        case AST_NAME: return 100;                       // Name
        case AST_NAME_TIME: return 101;                  // NameTime
        case AST_NAME_AVOGADRO: return 102;              // NameAvogadro
        case AST_INTEGER: return 200;                    // Integer
        case AST_REAL: return 201;                       // Real
        case AST_REAL_E: return 202;                     // RealE
        case AST_RATIONAL: return 203;                   // Rational
        case AST_CONSTANT_E: return 300;                 // ConstantE
        case AST_CONSTANT_FALSE: return 301;             // ConstantFalse
        case AST_CONSTANT_PI: return 302;                // ConstantPi
        case AST_CONSTANT_TRUE: return 303;              // ConstantTrue
        case AST_PLUS: return 400;                       // Plus
        case AST_MINUS: return 401;                      // Minus
        case AST_TIMES: return 402;                      // Times
        case AST_DIVIDE: return 403;                     // Divide
        case AST_POWER: return 404;                      // Power
        case AST_FUNCTION: return 500;                   // Function
        case AST_FUNCTION_ABS: return 600;               // FunctionAbs
        case AST_FUNCTION_ARCCOS: return 601;            // FunctionArccos
        case AST_FUNCTION_ARCCOSH: return 602;           // FunctionArccosh
        case AST_FUNCTION_ARCCOT: return 603;            // FunctionArccot
        case AST_FUNCTION_ARCCOTH: return 604;           // FunctionArccoth
        case AST_FUNCTION_ARCCSC: return 605;            // FunctionArccsc
        case AST_FUNCTION_ARCCSCH: return 606;           // FunctionArccsch
        case AST_FUNCTION_ARCSEC: return 607;            // FunctionArcsec
        case AST_FUNCTION_ARCSECH: return 608;           // FunctionArcsech
        case AST_FUNCTION_ARCSIN: return 609;            // FunctionArcsin
        case AST_FUNCTION_ARCSINH: return 610;           // FunctionArcsinh
        case AST_FUNCTION_ARCTAN: return 611;            // FunctionArctan
        case AST_FUNCTION_ARCTANH: return 612;           // FunctionArctanh
        case AST_FUNCTION_CEILING: return 613;           // FunctionCeiling
        case AST_FUNCTION_COS: return 614;               // FunctionCos
        case AST_FUNCTION_COSH: return 615;              // FunctionCosh
        case AST_FUNCTION_COT: return 616;               // FunctionCot
        case AST_FUNCTION_COTH: return 617;              // FunctionCoth
        case AST_FUNCTION_CSC: return 618;               // FunctionCsc
        case AST_FUNCTION_CSCH: return 619;              // FunctionCsch
        case AST_FUNCTION_DELAY: return 620;             // FunctionDelay
        case AST_FUNCTION_EXP: return 621;               // FunctionExp
        case AST_FUNCTION_FACTORIAL: return 622;         // FunctionFactorial
        case AST_FUNCTION_FLOOR: return 623;             // FunctionFloor
        case AST_FUNCTION_LN: return 624;                // FunctionLn
        case AST_FUNCTION_LOG: return 625;               // FunctionLog
        case AST_FUNCTION_MAX: return 626;               // FunctionMax
        case AST_FUNCTION_MIN: return 627;               // FunctionMin
        case AST_FUNCTION_PIECEWISE: return 628;         // FunctionPiecewise
        case AST_FUNCTION_POWER: return 629;             // FunctionPower
        case AST_FUNCTION_QUOTIENT: return 630;          // FunctionQuotient
        case AST_FUNCTION_RATE_OF: return 631;           // FunctionRateOf
        case AST_FUNCTION_REM: return 632;               // FunctionRem
        case AST_FUNCTION_ROOT: return 633;              // FunctionRoot
        case AST_FUNCTION_SEC: return 634;               // FunctionSec
        case AST_FUNCTION_SECH: return 635;              // FunctionSech
        case AST_FUNCTION_SIN: return 636;               // FunctionSin
        case AST_FUNCTION_SINH: return 637;              // FunctionSinh
        case AST_FUNCTION_TAN: return 638;               // FunctionTan
        case AST_FUNCTION_TANH: return 639;              // FunctionTanh
        case AST_LAMBDA: return 700;                     // Lambda
        case AST_RELATIONAL_EQ: return 800;              // RelationalEq
        case AST_RELATIONAL_GEQ: return 801;             // RelationalGeq
        case AST_RELATIONAL_GT: return 802;              // RelationalGt
        case AST_RELATIONAL_LEQ: return 803;             // RelationalLeq
        case AST_RELATIONAL_LT: return 804;              // RelationalLt
        case AST_RELATIONAL_NEQ: return 805;             // RelationalNeq
        case AST_LOGICAL_AND: return 900;                // LogicalAnd
        case AST_LOGICAL_NOT: return 901;                // LogicalNot
        case AST_LOGICAL_OR: return 902;                 // LogicalOr
        case AST_LOGICAL_XOR: return 903;                // LogicalXor
        case AST_DISTRIB_FUNCTION_BERNOULLI: return 1000;    // DistribBernoulli
        case AST_DISTRIB_FUNCTION_BINOMIAL: return 1001;     // DistribBinomial
        case AST_DISTRIB_FUNCTION_CAUCHY: return 1002;       // DistribCauchy
        case AST_DISTRIB_FUNCTION_CHISQUARE: return 1003;    // DistribChisquare
        case AST_DISTRIB_FUNCTION_EXPONENTIAL: return 1004;  // DistribExponential
        case AST_DISTRIB_FUNCTION_GAMMA: return 1005;        // DistribGamma
        case AST_DISTRIB_FUNCTION_LAPLACE: return 1006;      // DistribLaplace
        case AST_DISTRIB_FUNCTION_LOGNORMAL: return 1007;    // DistribLognormal
        case AST_DISTRIB_FUNCTION_NORMAL: return 1008;       // DistribNormal
        case AST_DISTRIB_FUNCTION_POISSON: return 1009;      // DistribPoisson
        case AST_DISTRIB_FUNCTION_RAYLEIGH: return 1010;     // DistribRayleigh
        case AST_DISTRIB_FUNCTION_UNIFORM: return 1011;      // DistribUniform
        default: return 0;                               // Other
    }
}

/// The number of children of a node.
unsigned int s4h_ast_num_children(ASTNode* node) {
    return node->getNumChildren();
}

/// Child `n` of a node, null when there is none.
///
/// The child is owned by its parent and is freed with the tree.
ASTNode* s4h_ast_child(ASTNode* node, unsigned int n) {
    return node->getChild(n);
}

/// The name of a node, borrowed, null when it has none.
///
/// `ASTNode::getName()` answers a `const char*` (math/ASTNode.h), which is the
/// name the node holds itself or a string of a static table of libsbml, so it
/// is borrowed and no copy. It is the identifier of a `ci`, the text of a
/// csymbol, the name of a call of a function of the model, and otherwise the
/// name libsbml gives a built-in function, constant or lambda of its own; it is
/// null for an operator and for a number.
const char* s4h_ast_name(ASTNode* node) {
    return node->getName();
}

/// The number of a node, 0 when the node is no number.
///
/// What it answers says which of its out parameters it wrote: 1 the `integer`,
/// 2 the `real`, 3 the `real` and the `exponent` of a number in e-notation,
/// and 4 the `integer` and the `denominator` of a rational, whose numerator
/// libsbml answers as its integer. The parts of a number in e-notation and of
/// a rational stay apart, since `ASTNode::getValue` and `ASTNode::getReal`
/// multiply or divide them out and lose the form the document wrote.
int s4h_ast_number(ASTNode* node, long long* integer, double* real, long long* exponent,
                   long long* denominator) {
    switch (node->getType()) {
        case AST_INTEGER:
            *integer = static_cast<long long>(node->getInteger());
            return 1;
        case AST_REAL:
            *real = node->getReal();
            return 2;
        case AST_REAL_E:
            *real = node->getMantissa();
            *exponent = static_cast<long long>(node->getExponent());
            return 3;
        case AST_RATIONAL:
            *integer = static_cast<long long>(node->getNumerator());
            *denominator = static_cast<long long>(node->getDenominator());
            return 4;
        default:
            return 0;
    }
}

/// The `sbml:units` of a number node, handed over, null when they are not set.
///
/// `ASTNode::getUnits()` answers a `std::string` by value (math/ASTNode.h), a
/// temporary which dies at the end of this expression, so the units are copied
/// out with `handed_over` and never borrowed. `ASTNode::isSetUnits()` is that
/// the string is not empty, so the null of this says the same and the shim
/// needs no predicate of its own.
char* s4h_ast_units(ASTNode* node) {
    try {
        return node->isSetUnits() ? handed_over(node->getUnits()) : nullptr;
    } catch (...) {
        return nullptr;
    }
}

/* ---------------------------------------------------------------- registry */

/// The number of packages the registry of libsbml knows.
unsigned int s4h_registry_num_packages() {
    return SBMLExtensionRegistry::getNumRegisteredPackages();
}

/// The name of registered package `n`, handed over, null when there is none.
///
/// `SBMLExtensionRegistry::getRegisteredPackageName(unsigned int)` answers a
/// `std::string` by value (SBMLExtensionRegistry.h), so it is copied out with
/// `handed_over` and never borrowed.
char* s4h_registry_package_name(unsigned int n) {
    try {
        return handed_over(SBMLExtensionRegistry::getRegisteredPackageName(n));
    } catch (...) {
        return nullptr;
    }
}

}  // extern "C"
