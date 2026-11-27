//! Maps the document model onto the report.

use sbml4humans_model::{
    Ast, Compartment, Constraint, Document, Element, Event, EventAssignment, FunctionDefinition,
    InitialAssignment, KineticLaw, LocalParameter, MathElement, Model, ModifierSpeciesReference,
    Parameter, Reaction, Rule, RuleKind, Species, SpeciesReference, Trigger, Unit, UnitDefinition,
};

use crate::{
    Double, LinkGraph, Math, Package, Report, ReportCompartment, ReportConstraint, ReportDelay,
    ReportDocument, ReportEvent, ReportEventAssignment, ReportFunctionDefinition,
    ReportInitialAssignment, ReportKineticLaw, ReportLocalParameter, ReportModel,
    ReportModifierSpeciesReference, ReportParameter, ReportPriority, ReportReaction, ReportRule,
    ReportSpecies, ReportSpeciesReference, ReportTrigger, ReportUnit, ReportUnitDefinition, SBase,
    pk,
};

/// The scope and the key python gives the document, the one element keyed by a fixed name.
const DOCUMENT_KEY: &str = "document";

/// The key python gives a model which carries neither an id nor a metaId (`MAIN_MODEL_KEY`).
const MAIN_MODEL_KEY: &str = "model";

/// An object of the document model which is one element of SBML of one type.
///
/// Every list of a model is keyed by one call per list, which reaches the fields of `SBase` of a
/// child whatever type that list holds. The types of the model share no base of their own, each
/// holding those fields in a field `element`, so this is what they are asked for in common.
///
/// The type carries the name of its class as well, because that name decides two things of one
/// element: the `sbmlType` of its report object and, through `pk::element_name`, the place which
/// keys it where it carries no identifier. Naming it once is what keeps the two the same: a place
/// prefix which does not match the type is a key no reader of the report can follow, and no test
/// of a document whose every element of that type carries an id would see it.
trait Keyed {
    /// The name of the class of the element in SBML, which is the `sbmlType` of its report object.
    const SBML_TYPE: &'static str;

    /// The fields of `SBase` the object carries.
    fn element(&self) -> &Element;
}

/// Implements [`Keyed`] for every type of the model which holds its `SBase` in a field `element`.
///
/// The name is written out next to the type and not taken from it with `stringify!`, although the
/// two read the same on every line below: the name is a word of SBML and of the wire contract the
/// frontend reads, and a rename of a struct of this workspace may not move it.
macro_rules! keyed {
    ($($type:ty => $sbml_type:literal),* $(,)?) => {
        $(
            impl Keyed for $type {
                const SBML_TYPE: &'static str = $sbml_type;

                fn element(&self) -> &Element {
                    &self.element
                }
            }
        )*
    };
}

keyed!(
    Compartment => "Compartment",
    Constraint => "Constraint",
    Event => "Event",
    FunctionDefinition => "FunctionDefinition",
    InitialAssignment => "InitialAssignment",
    Parameter => "Parameter",
    Reaction => "Reaction",
    Species => "Species",
    UnitDefinition => "UnitDefinition",
);

/// The key of every child of one list of a model, each named by its place in that list.
///
/// The children of a list are all of one type but the rules, whose three classes `model` counts
/// apart and which therefore call [`pk::indexed`] for themselves.
fn places<T: Keyed>(children: &[T]) -> Vec<String> {
    pk::indexed(children, |child| child.element(), |_| T::SBML_TYPE)
}

/// The shared fields of an element, keyed in `scope` under `key`.
///
/// It is the one place of the builder which makes an `SBase`, through [`SBase::new`], which takes
/// the type of the element: a report object of no `sbmlType` is one the frontend cannot read.
pub(crate) fn sbase(scope: &str, sbml_type: &'static str, element: &Element, key: &str) -> SBase {
    let mut base = SBase {
        id: element.id.clone(),
        meta_id: element.meta_id.clone(),
        name: element.name.clone(),
        sbo: element.sbo.clone(),
        notes: element.notes.clone(),
        xml: element.xml.clone(),
        cvterms: cvterms(element),
        history: element.history.as_ref().map(|history| crate::ModelHistory {
            creators: history
                .creators
                .iter()
                .map(|creator| crate::Creator {
                    given_name: creator.given_name.clone(),
                    family_name: creator.family_name.clone(),
                    organization: creator.organization.clone(),
                    email: creator.email.clone(),
                })
                .collect(),
            created_date: history.created_date.clone(),
            modified_dates: history.modified_dates.clone(),
        }),
        ..SBase::new(pk::of(scope, sbml_type, key), sbml_type)
    };
    crate::packages::extend(&mut base, element, scope, key);
    base
}

/// Copy a qualified relation without using a call frame for every nested qualifier.
fn cvterm(term: &sbml4humans_model::CvTerm) -> crate::CVTerm {
    fn shallow(term: &sbml4humans_model::CvTerm) -> crate::CVTerm {
        crate::CVTerm {
            qualifier: term.qualifier.clone(),
            resources: term.resources.clone(),
            nested: vec![],
        }
    }
    let mut stack = vec![(term, 0, shallow(term))];
    while let Some((source, next, result)) = stack.pop() {
        if let Some(child) = source.nested.get(next) {
            stack.push((source, next + 1, result));
            stack.push((child, 0, shallow(child)));
        } else if let Some(parent) = stack.last_mut() {
            parent.2.nested.push(result);
        } else {
            return result;
        }
    }
    crate::CVTerm::default()
}

/// The authored annotations, with the same synthetic SBO relation as Python.
fn cvterms(element: &Element) -> Vec<crate::CVTerm> {
    let mut terms: Vec<_> = element.cvterms.iter().map(cvterm).collect();
    if let Some(sbo) = &element.sbo
        && !terms
            .iter()
            .flat_map(|term| &term.resources)
            .any(|resource| resource.contains(sbo))
    {
        terms.insert(
            0,
            crate::CVTerm {
                qualifier: "BQB_IS".to_owned(),
                resources: vec![format!("https://identifiers.org/{sbo}")],
                nested: vec![],
            },
        );
    }
    terms
}

/// The key of an element whose aliased id names what it sets: an initial assignment or a rule.
///
/// Python reads the `id` attribute of the document first, then the target libsbml aliases to the id
/// of the element (`KEYED_BY_TARGET_CLASSES`, `sbmlinfo.py:576-577`), then the metaId, then the key
/// its parent derived for it. The target comes before the metaId, which is the order of `_key`
/// (`sbmlinfo.py:572-581`) and not the one of [`pk::key`]: a model holds at most one initial
/// assignment per symbol and at most one rule per variable (core §4.8.2, §4.9.1), so the target
/// keys the element, and it keeps the permalink of a rule of a Level 2 document, which can carry no
/// id at all, independent of the metaId.
///
/// `target` is `None` for a target the document does not name, which is every algebraic rule and
/// an assignment of an invalid document, and `place` is the key the list of the parent derived,
/// which `pk::indexed` already answers as the identifier of an element which carries one: the
/// metaId of such an element is reserved against the places of its siblings although the target
/// and not the metaId keys it.
fn key_of_target(element: &Element, target: Option<&str>, place: &str) -> String {
    element
        .id
        .as_deref()
        .or(target)
        .or(element.meta_id.as_deref())
        .map_or_else(|| place.to_owned(), str::to_owned)
}

/// The type of the report object of a rule, which is the class of its kind in SBML.
const fn rule_type(kind: RuleKind) -> &'static str {
    match kind {
        RuleKind::Assignment => "AssignmentRule",
        RuleKind::Rate => "RateRule",
        RuleKind::Algebraic => "AlgebraicRule",
    }
}

/// The math of an element, a failure to write one named by the pk of the element which holds it.
///
/// # Errors
///
/// [`BuildError::Math`] for a tree no writer of [`crate::math`] writes, which the filler refuses to
/// hand over: it exists so that nothing here unwraps a value a document controls.
fn math_of(ast: Option<&Ast>, pk: &str) -> Result<Option<Math>, BuildError> {
    ast.map(crate::math::math)
        .transpose()
        .map_err(|source| BuildError::Math {
            pk: pk.to_owned(),
            source,
        })
}

/// One factor of a unit definition, a nested object of that definition with no key of its own.
fn unit(u: &Unit) -> ReportUnit {
    ReportUnit {
        kind: u.kind.clone(),
        exponent: u.exponent.map(Double),
        scale: u.scale.map(i64::from),
        multiplier: u.multiplier.map(Double),
    }
}

/// Why the report of a document could not be built.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BuildError {
    /// A unit factor the Python registry cannot render.
    #[error(transparent)]
    Units(#[from] crate::units::UnitError),
    /// The math of an element could not be written.
    ///
    /// It is unreachable by construction, since a filler of the model refuses a math which is
    /// nested too deep and one which holds a node of a kind no writer writes, and the whole read
    /// of the document fails with it. It is an error of the builder and no panic of it so that a
    /// later filler, of a document of any shape, cannot bring the process down.
    #[error("the math of {pk} cannot be written: {source}")]
    Math {
        /// The pk of the element which holds the math.
        pk: String,
        /// Why no writer writes the tree.
        source: crate::math::MathError,
    },
}

/// A compartment of a model.
fn compartment(scope: &str, key: &str, c: &Compartment) -> ReportCompartment {
    ReportCompartment {
        sbase: sbase(scope, Compartment::SBML_TYPE, &c.element, key),
        spatial_dimensions: c.spatial_dimensions.map(Double),
        size: c.size.map(Double),
        constant: c.constant,
        units: c.units.clone(),
        units_latex: None,
        derived_units: None,
    }
}

/// A species of a model.
fn species(scope: &str, key: &str, s: &Species) -> ReportSpecies {
    ReportSpecies {
        sbase: sbase(scope, Species::SBML_TYPE, &s.element, key),
        compartment: s.compartment.clone(),
        initial_amount: s.initial_amount.map(Double),
        initial_concentration: s.initial_concentration.map(Double),
        substance_units: s.substance_units.clone(),
        has_only_substance_units: s.has_only_substance_units,
        boundary_condition: s.boundary_condition,
        constant: s.constant,
        units_latex: None,
        derived_units: None,
        conversion_factor: None,
        fbc: None,
    }
}

/// A global parameter of a model.
fn parameter(scope: &str, key: &str, p: &Parameter) -> ReportParameter {
    ReportParameter {
        sbase: sbase(scope, Parameter::SBML_TYPE, &p.element, key),
        value: p.value.map(Double),
        constant: p.constant,
        units: p.units.clone(),
        units_latex: None,
        derived_units: None,
    }
}

/// A function definition with its lambda.
///
/// # Errors
///
/// What [`math_of`] answers for the math of the definition.
fn function_definition(
    scope: &str,
    key: &str,
    fd: &FunctionDefinition,
) -> Result<ReportFunctionDefinition, BuildError> {
    let sbase = sbase(scope, FunctionDefinition::SBML_TYPE, &fd.element, key);
    let math = math_of(fd.math.as_ref(), &sbase.pk)?;
    Ok(ReportFunctionDefinition { sbase, math })
}

/// A unit definition with its factors.
fn unit_definition(scope: &str, key: &str, ud: &UnitDefinition) -> ReportUnitDefinition {
    ReportUnitDefinition {
        sbase: sbase(scope, UnitDefinition::SBML_TYPE, &ud.element, key),
        units_latex: None,
        list_of_units: ud.units.iter().map(unit).collect(),
    }
}

/// An initial assignment of a symbol.
///
/// # Errors
///
/// What [`math_of`] answers for the math of the assignment.
fn initial_assignment(
    scope: &str,
    place: &str,
    ia: &InitialAssignment,
) -> Result<ReportInitialAssignment, BuildError> {
    // the symbol is a required attribute, so the empty string is a document which names none
    let symbol = (!ia.symbol.is_empty()).then_some(ia.symbol.as_str());
    let key = key_of_target(&ia.element, symbol, place);
    let sbase = sbase(scope, InitialAssignment::SBML_TYPE, &ia.element, &key);
    let math = math_of(ia.math.as_ref(), &sbase.pk)?;
    Ok(ReportInitialAssignment {
        sbase,
        symbol: ia.symbol.clone(),
        math,
        derived_units: None,
    })
}

/// A rule of one of the three rule classes of SBML.
///
/// `place` is the key the list of the rules gave this one, its place among the rules of its class
/// or an identifier it carries of its own, which the variable it sets still wins over.
///
/// # Errors
///
/// What [`math_of`] answers for the math of the rule.
fn rule(scope: &str, place: &str, r: &Rule) -> Result<ReportRule, BuildError> {
    let key = key_of_target(&r.element, r.variable.as_deref(), place);
    let sbase = sbase(scope, rule_type(r.kind), &r.element, &key);
    let math = math_of(r.math.as_ref(), &sbase.pk)?;
    Ok(ReportRule {
        sbase,
        // the class of the algebraic rule declares no variable, so its object carries no such
        // key; the two others declare a required string, which python writes as the empty
        // string for a document which names none
        variable: match r.kind {
            RuleKind::Algebraic => None,
            RuleKind::Assignment | RuleKind::Rate => Some(r.variable.clone().unwrap_or_default()),
        },
        math,
        derived_units: None,
    })
}

/// A constraint with its message.
///
/// A constraint carries nothing which names it, so `key`, the key the list of the constraints gave
/// it, is its place in that list wherever it carries no identifier of its own.
///
/// # Errors
///
/// What [`math_of`] answers for the math of the constraint.
fn constraint(scope: &str, key: &str, c: &Constraint) -> Result<ReportConstraint, BuildError> {
    let sbase = sbase(scope, Constraint::SBML_TYPE, &c.element, key);
    let math = math_of(c.math.as_ref(), &sbase.pk)?;
    Ok(ReportConstraint {
        sbase,
        math,
        message: c.message.clone(),
    })
}

/// Whether a reaction is reversible, with the default of Level 1 and Level 2.
///
/// A port of `_reversible` (`sbmlinfo.py:1137-1147`). Level 1 and every version of Level 2 make a
/// reaction reversible unless it says otherwise (L2V4 §4.13.1); Level 3 requires the attribute and
/// has no default, so a reaction of Level 3 which states nothing states nothing in the report.
fn reversible(r: &Reaction, level: u32) -> Option<bool> {
    r.reversible.or_else(|| (level < 3).then_some(true))
}

/// Whether a reaction is fast, with the default of its level and version.
///
/// A port of `_fast` (`sbmlinfo.py:1150-1162`). Level 1 and Level 2 from Version 2 on make a
/// reaction slow unless it says otherwise (L2V4 §4.13.1). Level 2 Version 1 defined no default,
/// Level 3 Version 1 requires the attribute and Level 3 Version 2 dropped it.
fn fast(r: &Reaction, level: u32, version: u32) -> Option<bool> {
    r.fast
        .or_else(|| (level == 1 || (level == 2 && version >= 2)).then_some(false))
}

/// The stoichiometry of a reference, with the default of Level 1 and Level 2.
///
/// A port of `_stoichiometry` (`sbmlinfo.py:1165-1176`). Level 1 and Level 2 define the default 1
/// for a reference which writes neither a number nor a `stoichiometryMath` (L2V4 §4.13.2), which
/// is the raw answer of libsbml the model carries. Level 3 has no default: an unset stoichiometry
/// is set by a rule or an initial assignment which names the id of the reference, or is unknown.
fn stoichiometry(sr: &SpeciesReference, level: u32) -> Option<Double> {
    if sr.stoichiometry_set || (level < 3 && !sr.stoichiometry_math) {
        return Some(Double(sr.stoichiometry));
    }
    None
}

/// The reactants or the products of one reaction, in the order of the document.
///
/// `side` is `reactant` or `product`, the word python keys a reference by together with the species
/// it is about; a reference which carries an id or a metaId is keyed by it instead, and a key which
/// two references of one side share is numbered by its occurrence, which is why the keys of the
/// whole list are derived at once (`pk::derived`).
fn species_references(
    scope: &str,
    reaction_key: &str,
    side: &str,
    list: &[SpeciesReference],
    level: u32,
) -> Vec<ReportSpeciesReference> {
    let keys = pk::derived(
        list,
        |sr| &sr.element,
        |sr| format!("{reaction_key}.{side}.{}", sr.species),
    );
    list.iter()
        .zip(keys)
        .map(|(sr, key)| ReportSpeciesReference {
            sbase: sbase(scope, "SpeciesReference", &sr.element, &key),
            species: sr.species.clone(),
            stoichiometry: stoichiometry(sr, level),
            constant: sr.constant,
        })
        .collect()
}

/// The modifiers of one reaction, in the order of the document.
///
/// They are keyed like the reactants and the products of [`species_references`], by the word
/// `modifier` and the species the modifier is about.
fn modifier_species_references(
    scope: &str,
    reaction_key: &str,
    list: &[ModifierSpeciesReference],
) -> Vec<ReportModifierSpeciesReference> {
    let keys = pk::derived(
        list,
        |m| &m.element,
        |m| format!("{reaction_key}.modifier.{}", m.species),
    );
    list.iter()
        .zip(keys)
        .map(|(m, key)| ReportModifierSpeciesReference {
            sbase: sbase(scope, "ModifierSpeciesReference", &m.element, &key),
            species: m.species.clone(),
        })
        .collect()
}

/// The key of every local parameter of one kinetic law, the one element whose own id never keys it.
///
/// A port of the keying of `kinetic_law` (`sbmlinfo.py:1345-1375`), which keys its parameters
/// itself rather than through `_keyed_by`. Python passes `use_id=False`: the id of a local
/// parameter is a genuine id, required by SBML and unique within its law (core §4.11.6), but scoped
/// to that law and not to the model, since two laws may each hold a parameter of the same id, so
/// the key carries the law instead. A law which leaves an id out is invalid and its parameter is
/// keyed by its place among the parameters, `J0.kineticLaw.localParameter.1`, and a law which
/// repeats an id has the repetition told apart by its occurrence, so that no two parameters of one
/// law share a pk. The metaId of a parameter, which is unique within the document, wins over that
/// key (`_key`, `sbmlinfo.py:578-581`) and is reserved against the keys of the siblings, which is
/// why every parameter takes a candidate and only the others keep it.
///
/// `law_key` is the key the reaction derived for its law and not the key the law is reported under:
/// a law with a metaId of its own is keyed by it and its parameters stay keyed by the reaction.
fn local_parameter_keys(law_key: &str, parameters: &[LocalParameter]) -> Vec<String> {
    let candidates: Vec<String> = parameters
        .iter()
        .enumerate()
        .map(|(index, lp)| match lp.element.id.as_deref() {
            Some(id) => format!("{law_key}.{id}"),
            None => format!("{law_key}.localParameter.{index}"),
        })
        .collect();
    let reserved: Vec<&str> = parameters
        .iter()
        .filter_map(|lp| lp.element.meta_id.as_deref())
        .collect();
    pk::unique(&candidates, &reserved)
        .into_iter()
        .zip(parameters)
        .map(|(key, lp)| lp.element.meta_id.clone().unwrap_or(key))
        .collect()
}

/// A local parameter of a kinetic law, keyed by [`local_parameter_keys`].
///
/// The `id` field of the report object carries the id of the parameter all the same.
fn local_parameter(scope: &str, key: &str, lp: &LocalParameter) -> ReportLocalParameter {
    ReportLocalParameter {
        sbase: sbase(scope, "LocalParameter", &lp.element, key),
        value: lp.value.map(Double),
        units: lp.units.clone(),
        units_latex: None,
        derived_units: None,
    }
}

/// The kinetic law of a reaction with its local parameters.
///
/// A law carries nothing which names it, so one without an identifier of its own is keyed
/// `<reaction key>.kineticLaw`.
///
/// # Errors
///
/// What [`math_of`] answers for the math of the law, its rate.
fn kinetic_law(
    scope: &str,
    reaction_key: &str,
    law: &KineticLaw,
) -> Result<ReportKineticLaw, BuildError> {
    let derived = format!("{reaction_key}.kineticLaw");
    let sbase = sbase(
        scope,
        "KineticLaw",
        &law.element,
        &pk::key(&law.element, || derived.clone()),
    );
    let math = math_of(law.math.as_ref(), &sbase.pk)?;
    Ok(ReportKineticLaw {
        sbase,
        math,
        derived_units: None,
        list_of_local_parameters: law
            .local_parameters
            .iter()
            .zip(local_parameter_keys(&derived, &law.local_parameters))
            .map(|(lp, key)| local_parameter(scope, &key, lp))
            .collect(),
    })
}

/// A reaction with its participants, its kinetic law and its readable equation.
///
/// `level` and `version` are the ones of the document, which decide what the report states of a
/// `reversible`, a `fast` and a stoichiometry the document itself does not state.
///
/// # Errors
///
/// What [`math_of`] answers for the math of the kinetic law of the reaction.
fn reaction(
    scope: &str,
    key: &str,
    level: u32,
    version: u32,
    r: &Reaction,
) -> Result<ReportReaction, BuildError> {
    let law = r
        .kinetic_law
        .as_ref()
        .map(|law| kinetic_law(scope, key, law))
        .transpose()?;
    Ok(ReportReaction {
        sbase: sbase(scope, Reaction::SBML_TYPE, &r.element, key),
        reversible: reversible(r, level),
        fast: fast(r, level, version),
        compartment: r.compartment.clone(),
        list_of_reactants: species_references(scope, key, "reactant", &r.reactants, level),
        list_of_products: species_references(scope, key, "product", &r.products, level),
        list_of_modifiers: modifier_species_references(scope, key, &r.modifiers),
        kinetic_law: law,
        equation: crate::equation::equation(r),
        fbc: None,
    })
}

/// The trigger of an event: the condition which fires it.
///
/// A trigger carries nothing which names it, so one without an identifier of its own is keyed
/// `<event key>.trigger`.
///
/// # Errors
///
/// What [`math_of`] answers for the math of the trigger.
fn trigger(scope: &str, event_key: &str, t: &Trigger) -> Result<ReportTrigger, BuildError> {
    let key = pk::key(&t.element, || format!("{event_key}.trigger"));
    let sbase = sbase(scope, "Trigger", &t.element, &key);
    let math = math_of(t.math.as_ref(), &sbase.pk)?;
    Ok(ReportTrigger {
        sbase,
        math,
        initial_value: t.initial_value,
        persistent: t.persistent,
    })
}

/// The shared fields of the priority and the delay of an event, which carry a math and nothing else.
///
/// `name` is the word python keys the element by below its event, which is the name it has in the
/// file, and the element is keyed by an identifier of its own where it carries one.
///
/// # Errors
///
/// What [`math_of`] answers for the math of the element.
fn math_element(
    scope: &str,
    sbml_type: &'static str,
    event_key: &str,
    name: &str,
    element: &MathElement,
) -> Result<(SBase, Option<Math>), BuildError> {
    let key = pk::key(&element.element, || format!("{event_key}.{name}"));
    let sbase = sbase(scope, sbml_type, &element.element, &key);
    let math = math_of(element.math.as_ref(), &sbase.pk)?;
    Ok((sbase, math))
}

/// The priority of an event: the order of the events of one moment, which Level 3 added.
///
/// # Errors
///
/// What [`math_of`] answers for the math of the priority.
fn priority(scope: &str, event_key: &str, p: &MathElement) -> Result<ReportPriority, BuildError> {
    let (sbase, math) = math_element(scope, "Priority", event_key, "priority", p)?;
    Ok(ReportPriority { sbase, math })
}

/// The delay of an event: the time between the trigger and the execution.
///
/// # Errors
///
/// What [`math_of`] answers for the math of the delay.
fn delay(scope: &str, event_key: &str, d: &MathElement) -> Result<ReportDelay, BuildError> {
    let (sbase, math) = math_element(scope, "Delay", event_key, "delay", d)?;
    Ok(ReportDelay { sbase, math })
}

/// The assignments of one event, in the order of the document.
///
/// An assignment is keyed by the variable it sets, which an event sets at most once, and the keys
/// of the whole list are derived at once so that an assignment of an invalid document which repeats
/// a variable is told apart from the first by its occurrence. libsbml aliases the id of an
/// assignment to its variable, so the id of the model is the `id` attribute of the document alone
/// (`ALIASED_ID_CLASSES`, `sbmlinfo.py:119-123`) and keys the assignment where the document writes
/// one.
///
/// # Errors
///
/// What [`math_of`] answers for the math of the first assignment whose math no writer writes.
fn event_assignments(
    scope: &str,
    event_key: &str,
    list: &[EventAssignment],
) -> Result<Vec<ReportEventAssignment>, BuildError> {
    let keys = pk::derived(
        list,
        |ea| &ea.element,
        |ea| format!("{event_key}.{}", ea.variable),
    );
    let mut assignments = Vec::with_capacity(list.len());
    for (ea, key) in list.iter().zip(keys) {
        let sbase = sbase(scope, "EventAssignment", &ea.element, &key);
        let math = math_of(ea.math.as_ref(), &sbase.pk)?;
        assignments.push(ReportEventAssignment {
            sbase,
            variable: ea.variable.clone(),
            math,
        });
    }
    Ok(assignments)
}

/// An event with its trigger, its priority, its delay and its assignments.
///
/// An event carries nothing which names it, so `key`, the key the list of the events gave it, is
/// its place in that list wherever it carries no identifier of its own, and everything below the
/// event is keyed after that key.
///
/// # Errors
///
/// What [`math_of`] answers for the math of the first element of the event whose math no writer
/// writes.
fn event(scope: &str, key: &str, e: &Event) -> Result<ReportEvent, BuildError> {
    Ok(ReportEvent {
        sbase: sbase(scope, Event::SBML_TYPE, &e.element, key),
        use_values_from_trigger_time: e.use_values_from_trigger_time,
        trigger: e
            .trigger
            .as_ref()
            .map(|t| trigger(scope, key, t))
            .transpose()?,
        priority: e
            .priority
            .as_ref()
            .map(|p| priority(scope, key, p))
            .transpose()?,
        delay: e.delay.as_ref().map(|d| delay(scope, key, d)).transpose()?,
        list_of_event_assignments: event_assignments(scope, key, &e.event_assignments)?,
    })
}

/// The report of one model, every element of it keyed in the scope of that model.
///
/// # Errors
///
/// What [`math_of`] answers, for the first element of the model whose math no writer writes.
///
/// `level` and `version` are the ones of the document, which a reaction of the model is reported
/// with the defaults of.
fn model(
    model: &Model,
    level: u32,
    version: u32,
    scope: &str,
    kind: &'static str,
) -> Result<ReportModel, BuildError> {
    // the key of the model is the scope of every element below it

    // every list of the model keys its children by their place, the whole list at once: an element
    // which carries an id or a metaId is keyed by it and the others by the name of their element
    // and their place among the children of that name (`_indexed`, `sbmlinfo.py:984-1019`). A rule
    // is counted under the name of its class, so an algebraic rule which carries no identifier is
    // keyed by its place among the algebraic rules alone, which an edit of another rule does not
    // move
    let rule_keys = pk::indexed(&model.rules, |r| &r.element, |r| rule_type(r.kind));
    let mut rules = Vec::with_capacity(model.rules.len());
    for (r, key) in model.rules.iter().zip(rule_keys) {
        rules.push(rule(scope, &key, r)?);
    }
    Ok(ReportModel {
        // the model is keyed by the key which is its own scope, python writes `<key>/Model:<key>`
        sbase: sbase(scope, "Model", &model.element, scope),
        kind,
        annotation_xml: model.element.annotation.clone(),
        substance_units: None,
        substance_units_latex: None,
        time_units: None,
        time_units_latex: None,
        volume_units: None,
        volume_units_latex: None,
        area_units: None,
        area_units_latex: None,
        length_units: None,
        length_units_latex: None,
        extent_units: None,
        extent_units_latex: None,
        conversion_factor: None,
        list_of_function_definitions: model
            .function_definitions
            .iter()
            .zip(places(&model.function_definitions))
            .map(|(fd, key)| function_definition(scope, &key, fd))
            .collect::<Result<Vec<_>, BuildError>>()?,
        list_of_unit_definitions: model
            .unit_definitions
            .iter()
            .zip(places(&model.unit_definitions))
            .map(|(ud, key)| unit_definition(scope, &key, ud))
            .collect(),
        list_of_compartments: model
            .compartments
            .iter()
            .zip(places(&model.compartments))
            .map(|(c, key)| compartment(scope, &key, c))
            .collect(),
        list_of_species: model
            .species
            .iter()
            .zip(places(&model.species))
            .map(|(s, key)| species(scope, &key, s))
            .collect(),
        list_of_parameters: model
            .parameters
            .iter()
            .zip(places(&model.parameters))
            .map(|(p, key)| parameter(scope, &key, p))
            .collect(),
        list_of_initial_assignments: model
            .initial_assignments
            .iter()
            .zip(places(&model.initial_assignments))
            .map(|(ia, key)| initial_assignment(scope, &key, ia))
            .collect::<Result<Vec<_>, BuildError>>()?,
        list_of_rules: rules,
        list_of_constraints: model
            .constraints
            .iter()
            .zip(places(&model.constraints))
            .map(|(c, key)| constraint(scope, &key, c))
            .collect::<Result<Vec<_>, BuildError>>()?,
        list_of_reactions: model
            .reactions
            .iter()
            .zip(places(&model.reactions))
            .map(|(r, key)| reaction(scope, &key, level, version, r))
            .collect::<Result<Vec<_>, BuildError>>()?,
        list_of_events: model
            .events
            .iter()
            .zip(places(&model.events))
            .map(|(e, key)| event(scope, &key, e))
            .collect::<Result<Vec<_>, BuildError>>()?,
        list_of_submodels: vec![],
        list_of_ports: vec![],
        list_of_gene_products: vec![],
        list_of_objectives: vec![],
        list_of_flux_bounds: vec![],
        list_of_user_defined_constraints: vec![],
        list_of_qualitative_species: vec![],
        list_of_transitions: vec![],
        fbc: None,
    })
}

/// Builds the report of a document. The walk is a flat loop over the lists, it never recurses.
///
/// # Errors
///
/// [`BuildError::Math`] for an element whose math no writer of [`crate::math`] writes, which a
/// filler of the model refuses to hand over: no caller has to handle it, and it is an error rather
/// than a panic so that no writer of the report unwraps a value a document controls.
pub fn build(document: &Document) -> Result<Report, BuildError> {
    let main_key = document.model.as_ref().map(|m| {
        pk::key(&m.element, || {
            document
                .main_model_fallback
                .clone()
                .unwrap_or_else(|| MAIN_MODEL_KEY.to_owned())
        })
    });
    let definition_keys = pk::keyed_by_reserved(
        &document.model_definitions,
        |m| &m.element,
        &document
            .model_definitions
            .iter()
            .enumerate()
            .map(|(i, _)| format!("modelDefinition.{i}"))
            .collect::<Vec<_>>(),
        &main_key.as_deref().into_iter().collect::<Vec<_>>(),
    );
    let mut report = Report {
        document: ReportDocument {
            // the document is the one element whose scope and key are both a fixed name,
            // python writes `document/SBMLDocument:document`
            sbase: sbase(
                DOCUMENT_KEY,
                "SBMLDocument",
                &document.element,
                DOCUMENT_KEY,
            ),
            level: i64::from(document.level),
            version: i64::from(document.version),
            packages: document
                .packages
                .iter()
                .map(|p| Package {
                    prefix: p.prefix.clone(),
                    version: i64::from(p.version),
                })
                .collect(),
            annotation_xml: document.element.annotation.clone(),
        },
        models: document
            .models()
            .enumerate()
            .map(|(i, m)| {
                let kind = if document.model.is_some() && i == 0 {
                    "model"
                } else {
                    "modelDefinition"
                };
                let fallback = if kind == "model" {
                    main_key.clone().unwrap_or_default()
                } else {
                    definition_keys[i - usize::from(document.model.is_some())].clone()
                };
                let scope = pk::key(&m.element, || fallback);
                model(m, document.level, document.version, &scope, kind)
            })
            .collect::<Result<Vec<_>, BuildError>>()?,
        external_model_definitions: crate::packages::external(&document.element),
        link_graph: LinkGraph::default(),
        ode_system: None,
        ode_error: None,
    };
    for (input, output) in document.models().zip(&mut report.models) {
        crate::packages::model(
            input,
            output,
            document.packages.iter().any(|p| p.name == "fbc"),
        );
    }
    crate::units::apply(document, &mut report)?;
    report.link_graph = crate::links::build(&report, document);
    crate::odes::apply(document, &mut report);
    Ok(report)
}
