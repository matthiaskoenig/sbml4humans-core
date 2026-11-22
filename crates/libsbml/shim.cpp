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

}  // extern "C"
