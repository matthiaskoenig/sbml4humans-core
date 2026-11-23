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

/// A node of a syntax tree of a document as the Rust side reads it.
///
/// Every `getMath` of libsbml answers a `const ASTNode*`, while the getters of
/// a node take an `ASTNode*`, since `ASTNode::getChild` is no const member.
/// The cast is sound: the node belongs to a document which is not const, and
/// every getter the reader of a math calls is a const member, so nothing here
/// and nothing in Rust writes to the tree.
ASTNode* readable(const ASTNode* node) {
    return const_cast<ASTNode*>(node);
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
/// `getId` is the `id` attribute of every class the filler reads apart from
/// the three which alias it to another attribute, the initial assignment, the
/// rule and the event assignment; those read `s4h_sbase_id_attribute`.
const char* s4h_sbase_id(SBase* sbase) {
    return borrowed(sbase->isSetId(), sbase->getId());
}

/// The `id` attribute of an element, null when it is not set.
///
/// libsbml aliases `getId()` of the initial assignment, of the rule and of the
/// event assignment to the attribute which names what they set, the `symbol`
/// or the `variable`, for compatibility with the levels in which those classes
/// carried no id. Their `id` attribute is this one, which is the id the report
/// of the python implementation carries of them (`sbmlinfo._identifier`).
/// `getIdAttribute` answers a `const std::string&` of the element, so it is
/// borrowed.
const char* s4h_sbase_id_attribute(SBase* sbase) {
    return borrowed(sbase->isSetIdAttribute(), sbase->getIdAttribute());
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

/// The number of function definitions of a model.
unsigned int s4h_model_num_function_definitions(SBase* sbase) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? 0 : model->getNumFunctionDefinitions();
}

/// Function definition `n` of a model, null when there is none.
SBase* s4h_model_function_definition(SBase* sbase, unsigned int n) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? nullptr : model->getFunctionDefinition(n);
}

/// The number of unit definitions of a model.
unsigned int s4h_model_num_unit_definitions(SBase* sbase) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? 0 : model->getNumUnitDefinitions();
}

/// Unit definition `n` of a model, null when there is none.
SBase* s4h_model_unit_definition(SBase* sbase, unsigned int n) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? nullptr : model->getUnitDefinition(n);
}

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

/// The number of initial assignments of a model.
unsigned int s4h_model_num_initial_assignments(SBase* sbase) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? 0 : model->getNumInitialAssignments();
}

/// Initial assignment `n` of a model, null when there is none.
SBase* s4h_model_initial_assignment(SBase* sbase, unsigned int n) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? nullptr : model->getInitialAssignment(n);
}

/// The number of rules of a model.
unsigned int s4h_model_num_rules(SBase* sbase) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? 0 : model->getNumRules();
}

/// Rule `n` of a model, null when there is none.
SBase* s4h_model_rule(SBase* sbase, unsigned int n) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? nullptr : model->getRule(n);
}

/// The number of constraints of a model.
unsigned int s4h_model_num_constraints(SBase* sbase) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? 0 : model->getNumConstraints();
}

/// Constraint `n` of a model, null when there is none.
SBase* s4h_model_constraint(SBase* sbase, unsigned int n) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? nullptr : model->getConstraint(n);
}

/// The number of reactions of a model.
unsigned int s4h_model_num_reactions(SBase* sbase) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? 0 : model->getNumReactions();
}

/// Reaction `n` of a model, null when there is none.
SBase* s4h_model_reaction(SBase* sbase, unsigned int n) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? nullptr : model->getReaction(n);
}

/// The number of events of a model.
unsigned int s4h_model_num_events(SBase* sbase) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? 0 : model->getNumEvents();
}

/// Event `n` of a model, null when there is none.
SBase* s4h_model_event(SBase* sbase, unsigned int n) {
    Model* model = dynamic_cast<Model*>(sbase);
    return model == nullptr ? nullptr : model->getEvent(n);
}

/* ----------------------------------------------------- function definition */

/// The `math` of a function definition, borrowed, null when it has none.
///
/// The tree belongs to the element and is freed with the document, so it is
/// never handed to `s4h_ast_free`.
ASTNode* s4h_function_definition_math(SBase* sbase) {
    FunctionDefinition* definition = dynamic_cast<FunctionDefinition*>(sbase);
    if (definition == nullptr || !definition->isSetMath()) {
        return nullptr;
    }
    return readable(definition->getMath());
}

/* --------------------------------------------------------- unit definition */

/// The number of units of a unit definition.
unsigned int s4h_unit_definition_num_units(SBase* sbase) {
    UnitDefinition* definition = dynamic_cast<UnitDefinition*>(sbase);
    return definition == nullptr ? 0 : definition->getNumUnits();
}

/// Unit `n` of a unit definition, null when there is none.
SBase* s4h_unit_definition_unit(SBase* sbase, unsigned int n) {
    UnitDefinition* definition = dynamic_cast<UnitDefinition*>(sbase);
    return definition == nullptr ? nullptr : definition->getUnit(n);
}

/* -------------------------------------------------------------------- unit */

/// The `kind` of a unit as the name of the base unit, null when it is not set.
///
/// libsbml answers the kind as the integer of its constant, and
/// `UnitKind_toString` answers the name of that constant as a static string of
/// libsbml (UnitKind.h), which outlives every document and is borrowed.
const char* s4h_unit_kind(SBase* sbase) {
    Unit* unit = dynamic_cast<Unit*>(sbase);
    if (unit == nullptr || !unit->isSetKind()) {
        return nullptr;
    }
    return UnitKind_toString(unit->getKind());
}

/// The `exponent` of a unit.
///
/// It is read as a double because Level 3 allows a fractional exponent (core
/// §4.4.2), which the integer getter of libsbml truncates.
int s4h_unit_exponent(SBase* sbase, double* value) {
    Unit* unit = dynamic_cast<Unit*>(sbase);
    if (unit == nullptr || !unit->isSetExponent()) {
        return 0;
    }
    *value = unit->getExponentAsDouble();
    return 1;
}

/// The `scale` of a unit.
int s4h_unit_scale(SBase* sbase, int* value) {
    Unit* unit = dynamic_cast<Unit*>(sbase);
    if (unit == nullptr || !unit->isSetScale()) {
        return 0;
    }
    *value = unit->getScale();
    return 1;
}

/// The `multiplier` of a unit.
int s4h_unit_multiplier(SBase* sbase, double* value) {
    Unit* unit = dynamic_cast<Unit*>(sbase);
    if (unit == nullptr || !unit->isSetMultiplier()) {
        return 0;
    }
    *value = unit->getMultiplier();
    return 1;
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

/* ------------------------------------------------------ initial assignment */

/// The `symbol` of an initial assignment, the element it sets, borrowed.
///
/// It is read unguarded, as the python implementation reads it
/// (`sbmlinfo.initial_assignment`): libsbml answers the empty string where the
/// document names nothing, which the Rust side reads as nothing.
const char* s4h_initial_assignment_symbol(SBase* sbase) {
    InitialAssignment* assignment = dynamic_cast<InitialAssignment*>(sbase);
    return assignment == nullptr ? nullptr : assignment->getSymbol().c_str();
}

/// The `math` of an initial assignment, borrowed, null when it has none.
ASTNode* s4h_initial_assignment_math(SBase* sbase) {
    InitialAssignment* assignment = dynamic_cast<InitialAssignment*>(sbase);
    if (assignment == nullptr || !assignment->isSetMath()) {
        return nullptr;
    }
    return readable(assignment->getMath());
}

/* -------------------------------------------------------------------- rule */

/// Which of the three rule classes a rule is.
///
/// 0 is an assignment rule, 1 a rate rule and 2 an algebraic rule, which is
/// the one of the three which sets no variable. -1 is an element which is no
/// rule and a rule of a class which is none of the three, which the python
/// implementation refuses the report of (`sbmlinfo.rule` raises `TypeError`).
/// A Level 1 rule carries its class in its `type` attribute, which the
/// predicates of libsbml read as well.
int s4h_rule_kind(SBase* sbase) {
    Rule* rule = dynamic_cast<Rule*>(sbase);
    if (rule == nullptr) {
        return -1;
    }
    if (rule->isAssignment()) {
        return 0;
    }
    if (rule->isRate()) {
        return 1;
    }
    if (rule->isAlgebraic()) {
        return 2;
    }
    return -1;
}

/// The `variable` of a rule, the element it sets, borrowed.
///
/// It is read unguarded, as the python implementation reads it
/// (`sbmlinfo.rule`): libsbml answers the empty string for an algebraic rule,
/// which sets no variable, and for a document which names none.
const char* s4h_rule_variable(SBase* sbase) {
    Rule* rule = dynamic_cast<Rule*>(sbase);
    return rule == nullptr ? nullptr : rule->getVariable().c_str();
}

/// The `math` of a rule, borrowed, null when it has none.
///
/// A Level 1 rule writes its math as the attribute `formula`, which
/// `Rule::getMath` parses on the first call and keeps in the element, so the
/// tree is owned by the element in every level and is never freed here.
ASTNode* s4h_rule_math(SBase* sbase) {
    Rule* rule = dynamic_cast<Rule*>(sbase);
    if (rule == nullptr || !rule->isSetMath()) {
        return nullptr;
    }
    return readable(rule->getMath());
}

/* -------------------------------------------------------------- constraint */

/// The `math` of a constraint, borrowed, null when it has none.
ASTNode* s4h_constraint_math(SBase* sbase) {
    Constraint* constraint = dynamic_cast<Constraint*>(sbase);
    if (constraint == nullptr || !constraint->isSetMath()) {
        return nullptr;
    }
    return readable(constraint->getMath());
}

/// The `message` of a constraint as XML, handed over, null when it has none.
///
/// `Constraint::getMessageString` answers a `std::string` by value
/// (Constraint.h), so the message is copied out and never borrowed.
char* s4h_constraint_message(SBase* sbase) {
    try {
        Constraint* constraint = dynamic_cast<Constraint*>(sbase);
        if (constraint == nullptr || !constraint->isSetMessage()) {
            return nullptr;
        }
        return handed_over(constraint->getMessageString());
    } catch (...) {
        return nullptr;
    }
}

/* ---------------------------------------------------------------- reaction */

/// The `reversible` of a reaction.
///
/// libsbml answers `getReversible()` as true for a reaction of every level
/// which does not set the attribute, which is the default of Level 1 and
/// Level 2 and no value at all in Level 3, so the shim answers whether the
/// document sets it and the report writes the default of the level itself.
int s4h_reaction_reversible(SBase* sbase, int* value) {
    Reaction* reaction = dynamic_cast<Reaction*>(sbase);
    if (reaction == nullptr || !reaction->isSetReversible()) {
        return 0;
    }
    *value = reaction->getReversible() ? 1 : 0;
    return 1;
}

/// The `fast` of a reaction, the attribute Level 3 Version 2 dropped.
int s4h_reaction_fast(SBase* sbase, int* value) {
    Reaction* reaction = dynamic_cast<Reaction*>(sbase);
    if (reaction == nullptr || !reaction->isSetFast()) {
        return 0;
    }
    *value = reaction->getFast() ? 1 : 0;
    return 1;
}

/// The `compartment` of a reaction, null when it is not set.
const char* s4h_reaction_compartment(SBase* sbase) {
    Reaction* reaction = dynamic_cast<Reaction*>(sbase);
    if (reaction == nullptr) {
        return nullptr;
    }
    return borrowed(reaction->isSetCompartment(), reaction->getCompartment());
}

/// The number of reactants of a reaction.
unsigned int s4h_reaction_num_reactants(SBase* sbase) {
    Reaction* reaction = dynamic_cast<Reaction*>(sbase);
    return reaction == nullptr ? 0 : reaction->getNumReactants();
}

/// Reactant `n` of a reaction, null when there is none.
SBase* s4h_reaction_reactant(SBase* sbase, unsigned int n) {
    Reaction* reaction = dynamic_cast<Reaction*>(sbase);
    return reaction == nullptr ? nullptr : reaction->getReactant(n);
}

/// The number of products of a reaction.
unsigned int s4h_reaction_num_products(SBase* sbase) {
    Reaction* reaction = dynamic_cast<Reaction*>(sbase);
    return reaction == nullptr ? 0 : reaction->getNumProducts();
}

/// Product `n` of a reaction, null when there is none.
SBase* s4h_reaction_product(SBase* sbase, unsigned int n) {
    Reaction* reaction = dynamic_cast<Reaction*>(sbase);
    return reaction == nullptr ? nullptr : reaction->getProduct(n);
}

/// The number of modifiers of a reaction.
unsigned int s4h_reaction_num_modifiers(SBase* sbase) {
    Reaction* reaction = dynamic_cast<Reaction*>(sbase);
    return reaction == nullptr ? 0 : reaction->getNumModifiers();
}

/// Modifier `n` of a reaction, null when there is none.
SBase* s4h_reaction_modifier(SBase* sbase, unsigned int n) {
    Reaction* reaction = dynamic_cast<Reaction*>(sbase);
    return reaction == nullptr ? nullptr : reaction->getModifier(n);
}

/// The kinetic law of a reaction, null when it has none.
SBase* s4h_reaction_kinetic_law(SBase* sbase) {
    Reaction* reaction = dynamic_cast<Reaction*>(sbase);
    if (reaction == nullptr || !reaction->isSetKineticLaw()) {
        return nullptr;
    }
    return reaction->getKineticLaw();
}

/* ------------------------------------------------------- species reference */

/// The `species` of a reference, borrowed.
///
/// It reads a reactant, a product and a modifier alike, since the attribute is
/// the one of `SimpleSpeciesReference`, the base class of the three (core
/// §4.13.2). It is read unguarded, as the python implementation reads it
/// (`sbmlinfo.species_reference`): libsbml answers the empty string where the
/// document names nothing, which the Rust side reads as nothing.
const char* s4h_species_reference_species(SBase* sbase) {
    SimpleSpeciesReference* reference = dynamic_cast<SimpleSpeciesReference*>(sbase);
    return reference == nullptr ? nullptr : reference->getSpecies().c_str();
}

/// The `stoichiometry` of a reference as libsbml answers it, whatever that is.
///
/// Unlike every other number of the shim this is no optional attribute: the
/// value is written for every reference, set or not, and
/// `s4h_species_reference_stoichiometry_set` says whether the document wrote
/// it. libsbml answers the default of the level of a reference which sets
/// nothing, `1.0` in Level 1 and Level 2 and `NaN` in Level 3, and the report
/// is built from the number and the two predicates together
/// (`sbmlinfo._stoichiometry`). 0 is an element which is no reactant and no
/// product, a modifier among them, which carries no stoichiometry at all.
int s4h_species_reference_stoichiometry(SBase* sbase, double* value) {
    SpeciesReference* reference = dynamic_cast<SpeciesReference*>(sbase);
    if (reference == nullptr) {
        return 0;
    }
    *value = reference->getStoichiometry();
    return 1;
}

/// Whether the document sets the `stoichiometry` of a reference.
int s4h_species_reference_stoichiometry_set(SBase* sbase) {
    SpeciesReference* reference = dynamic_cast<SpeciesReference*>(sbase);
    return reference != nullptr && reference->isSetStoichiometry() ? 1 : 0;
}

/// Whether a reference carries a `stoichiometryMath`, which Level 3 dropped.
int s4h_species_reference_stoichiometry_math(SBase* sbase) {
    SpeciesReference* reference = dynamic_cast<SpeciesReference*>(sbase);
    return reference != nullptr && reference->isSetStoichiometryMath() ? 1 : 0;
}

/// The `constant` of a reference.
int s4h_species_reference_constant(SBase* sbase, int* value) {
    SpeciesReference* reference = dynamic_cast<SpeciesReference*>(sbase);
    if (reference == nullptr || !reference->isSetConstant()) {
        return 0;
    }
    *value = reference->getConstant() ? 1 : 0;
    return 1;
}

/* ------------------------------------------------------------- kinetic law */

/// The `math` of a kinetic law, borrowed, null when it has none.
///
/// A Level 1 law writes its math as the attribute `formula`, which
/// `KineticLaw::getMath` parses on the first call and keeps in the element, so
/// the tree is owned by the element in every level and is never freed here.
ASTNode* s4h_kinetic_law_math(SBase* sbase) {
    KineticLaw* law = dynamic_cast<KineticLaw*>(sbase);
    if (law == nullptr || !law->isSetMath()) {
        return nullptr;
    }
    return readable(law->getMath());
}

/// The number of parameters of a kinetic law, of a document of any level.
///
/// Level 3 writes them as `localParameter` in a `listOfLocalParameters` and
/// libsbml answers them from `getNumLocalParameters`; Level 1 and Level 2 write
/// them as `parameter` in a `listOfParameters`, where libsbml keeps them as
/// `Parameter` objects of `getNumParameters` and leaves the list of the local
/// parameters empty (core §4.11.5, §4.11.6, `sbmlinfo._kinetic_law_list`).
unsigned int s4h_kinetic_law_num_parameters(SBase* sbase) {
    KineticLaw* law = dynamic_cast<KineticLaw*>(sbase);
    if (law == nullptr) {
        return 0;
    }
    return law->getLevel() >= 3 ? law->getNumLocalParameters() : law->getNumParameters();
}

/// Parameter `n` of a kinetic law, of a document of any level, null when there
/// is none.
///
/// It answers the class of the level, a `LocalParameter` of Level 3 and a
/// `Parameter` of Level 1 and Level 2, which the getters of a local parameter
/// read alike.
SBase* s4h_kinetic_law_parameter(SBase* sbase, unsigned int n) {
    KineticLaw* law = dynamic_cast<KineticLaw*>(sbase);
    if (law == nullptr) {
        return nullptr;
    }
    if (law->getLevel() >= 3) {
        return law->getLocalParameter(n);
    }
    return law->getParameter(n);
}

/* --------------------------------------------------------- local parameter */

/// The `value` of a parameter of a kinetic law.
///
/// `LocalParameter` derives from `Parameter` (LocalParameter.h), so the getter
/// of a parameter reads the `LocalParameter` of a Level 3 law and the
/// `Parameter` of a Level 1 or Level 2 law alike; this is that getter under the
/// name of the one class the report has of either.
int s4h_local_parameter_value(SBase* sbase, double* value) {
    return s4h_parameter_value(sbase, value);
}

/// The `units` of a parameter of a kinetic law, null when they are not set.
const char* s4h_local_parameter_units(SBase* sbase) {
    return s4h_parameter_units(sbase);
}

/* ------------------------------------------------------------------- event */

/// The `useValuesFromTriggerTime` of an event.
int s4h_event_use_values_from_trigger_time(SBase* sbase, int* value) {
    Event* event = dynamic_cast<Event*>(sbase);
    if (event == nullptr || !event->isSetUseValuesFromTriggerTime()) {
        return 0;
    }
    *value = event->getUseValuesFromTriggerTime() ? 1 : 0;
    return 1;
}

/// The trigger of an event, null when it has none.
SBase* s4h_event_trigger(SBase* sbase) {
    Event* event = dynamic_cast<Event*>(sbase);
    if (event == nullptr || !event->isSetTrigger()) {
        return nullptr;
    }
    return event->getTrigger();
}

/// The priority of an event, null when it has none.
SBase* s4h_event_priority(SBase* sbase) {
    Event* event = dynamic_cast<Event*>(sbase);
    if (event == nullptr || !event->isSetPriority()) {
        return nullptr;
    }
    return event->getPriority();
}

/// The delay of an event, null when it has none.
SBase* s4h_event_delay(SBase* sbase) {
    Event* event = dynamic_cast<Event*>(sbase);
    if (event == nullptr || !event->isSetDelay()) {
        return nullptr;
    }
    return event->getDelay();
}

/// The number of assignments of an event.
unsigned int s4h_event_num_event_assignments(SBase* sbase) {
    Event* event = dynamic_cast<Event*>(sbase);
    return event == nullptr ? 0 : event->getNumEventAssignments();
}

/// Assignment `n` of an event, null when there is none.
SBase* s4h_event_event_assignment(SBase* sbase, unsigned int n) {
    Event* event = dynamic_cast<Event*>(sbase);
    return event == nullptr ? nullptr : event->getEventAssignment(n);
}

/* ----------------------------------------------------------------- trigger */

/// The `math` of a trigger, borrowed, null when it has none.
ASTNode* s4h_trigger_math(SBase* sbase) {
    Trigger* trigger = dynamic_cast<Trigger*>(sbase);
    if (trigger == nullptr || !trigger->isSetMath()) {
        return nullptr;
    }
    return readable(trigger->getMath());
}

/// The `initialValue` of a trigger.
int s4h_trigger_initial_value(SBase* sbase, int* value) {
    Trigger* trigger = dynamic_cast<Trigger*>(sbase);
    if (trigger == nullptr || !trigger->isSetInitialValue()) {
        return 0;
    }
    *value = trigger->getInitialValue() ? 1 : 0;
    return 1;
}

/// The `persistent` of a trigger.
int s4h_trigger_persistent(SBase* sbase, int* value) {
    Trigger* trigger = dynamic_cast<Trigger*>(sbase);
    if (trigger == nullptr || !trigger->isSetPersistent()) {
        return 0;
    }
    *value = trigger->getPersistent() ? 1 : 0;
    return 1;
}

/* ------------------------------------------------------------ math element */

/// The `math` of a priority or of a delay, borrowed, null when it has none.
///
/// Both classes derive from `SBase` directly and neither shares a base which
/// declares `getMath`, so the one function the filler of the two asks tries
/// each of them.
ASTNode* s4h_math_element_math(SBase* sbase) {
    Priority* priority = dynamic_cast<Priority*>(sbase);
    if (priority != nullptr) {
        return priority->isSetMath() ? readable(priority->getMath()) : nullptr;
    }
    Delay* delay = dynamic_cast<Delay*>(sbase);
    if (delay != nullptr) {
        return delay->isSetMath() ? readable(delay->getMath()) : nullptr;
    }
    return nullptr;
}

/* -------------------------------------------------------- event assignment */

/// The `variable` of an event assignment, the element it sets, borrowed.
///
/// It is read unguarded, as the python implementation reads it
/// (`sbmlinfo.event`): libsbml answers the empty string where the document
/// names nothing, which the Rust side reads as nothing.
const char* s4h_event_assignment_variable(SBase* sbase) {
    EventAssignment* assignment = dynamic_cast<EventAssignment*>(sbase);
    return assignment == nullptr ? nullptr : assignment->getVariable().c_str();
}

/// The `math` of an event assignment, borrowed, null when it has none.
ASTNode* s4h_event_assignment_math(SBase* sbase) {
    EventAssignment* assignment = dynamic_cast<EventAssignment*>(sbase);
    if (assignment == nullptr || !assignment->isSetMath()) {
        return nullptr;
    }
    return readable(assignment->getMath());
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

/// Reads the syntax tree of a `formula`, the math as Level 1 writes it.
///
/// The caller owns the tree and frees it with `s4h_ast_free`: this is the very
/// call `KineticLaw::getMath` makes of the `formula` of a kinetic law of a
/// Level 1 document, which keeps the tree in a member its destructor deletes
/// (KineticLaw.cpp), and the examples of libsbml free the tree of it with
/// `ASTNode_free` (examples/c++/translateMath.cpp). Null says that the string
/// is no formula.
ASTNode* s4h_read_formula(const char* formula) {
    try {
        return SBML_parseFormula(formula);
    } catch (...) {
        return nullptr;
    }
}

/// Frees a syntax tree of `s4h_read_math_from_string` or `s4h_read_formula`
/// with its children.
void s4h_ast_free(ASTNode* node) {
    delete node;
}

/// The kind of a node as the code of the `AstKind` of the report.
///
/// The codes are the contract with `sbml4humans_model::AstKind`, whose name is
/// the comment of every case. A type which is named by no case is 0, the kind
/// `Other`, which the reader of a math refuses: the kind of a type the report
/// does not know is never guessed to be a near one. A null node is `Other` as
/// well, so that a node the shim does not answer cannot end the process here:
/// every getter of a node answers its empty value for null rather than
/// dereferencing it.
///
/// Three groups of `ASTNodeType_t` stand outside the math of SBML, measured
/// against 5.21.2. `AST_CSYMBOL_FUNCTION` is a case: it is a csymbol of a
/// definition URL no package of libsbml registers, which carries the text of
/// the document and which libsbml and the stylesheet of the latex write as a
/// call of that text, so the report has a kind of it. `AST_UNKNOWN`, which
/// libsbml answers of an empty `math` element, is `Other`: the python
/// implementation writes the formula `()` and a latex of one space for it,
/// which no writer here has a kind to reach, so a document which holds one is
/// refused rather than reported wrongly. The quantifier, linear algebra,
/// statistics, series, algebra, relation, calculus, set theory, constant and
/// basic content types are `Other` and no document reaches them at all:
/// `readMathMLFromString` answers none of them and `writeMathMLToString` writes
/// no MathML of one, so the python implementation raises rather than writing a
/// formula or a latex which this could be faithful to.
int s4h_ast_kind(ASTNode* node) {
    if (node == nullptr) {
        return 0;  // Other
    }
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
        case AST_CSYMBOL_FUNCTION: return 501;           // CsymbolFunction
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
        case AST_LOGICAL_IMPLIES: return 904;            // LogicalImplies
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

/// The number of children of a node, 0 for a null node.
unsigned int s4h_ast_num_children(ASTNode* node) {
    return node == nullptr ? 0 : node->getNumChildren();
}

/// Child `n` of a node, null when there is none and for a null node.
///
/// The child is owned by its parent and is freed with the tree.
ASTNode* s4h_ast_child(ASTNode* node, unsigned int n) {
    return node == nullptr ? nullptr : node->getChild(n);
}

/// The name of a node, borrowed, null when it has none.
///
/// `ASTNode::getName()` answers a `const char*` (math/ASTNode.h), which is the
/// name the node holds itself or a string of a static table of libsbml, so it
/// is borrowed and no copy. It is the identifier of a `ci`, the text of a
/// csymbol, the name of a call of a function of the model, and otherwise the
/// name libsbml gives a built-in function, constant or lambda of its own; it is
/// null for an operator, for a number and for a null node.
const char* s4h_ast_name(ASTNode* node) {
    return node == nullptr ? nullptr : node->getName();
}

/// The number of a node, 0 when the node is no number and for a null node.
///
/// What it answers says which of its out parameters it wrote: 1 the `integer`,
/// 2 the `real`, 3 the `real` and the `exponent` of a number in e-notation,
/// and 4 the `integer` and the `denominator` of a rational, whose numerator
/// libsbml answers as its integer. The parts of a number in e-notation and of
/// a rational stay apart, since `ASTNode::getValue` and `ASTNode::getReal`
/// multiply or divide them out and lose the form the document wrote.
int s4h_ast_number(ASTNode* node, long long* integer, double* real, long long* exponent,
                   long long* denominator) {
    if (node == nullptr) {
        return 0;
    }
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

/// The `sbml:units` of a number node, handed over, null when they are not set
/// and for a null node.
///
/// `ASTNode::getUnits()` answers a `std::string` by value (math/ASTNode.h), a
/// temporary which dies at the end of this expression, so the units are copied
/// out with `handed_over` and never borrowed. `ASTNode::isSetUnits()` is that
/// the string is not empty, so the null of this says the same and the shim
/// needs no predicate of its own.
char* s4h_ast_units(ASTNode* node) {
    try {
        if (node == nullptr) {
            return nullptr;
        }
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
