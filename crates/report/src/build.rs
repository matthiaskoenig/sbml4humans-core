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

/// The derived units python writes when no unit is derived, deriving a unit belongs to a later slice.
///
/// The builder writes it for every element whose class of `model.py` declares the field, the
/// compartment, the species, the parameter, the initial assignment and the three rules, which is
/// wrong for a document which sets units at all. It is the one value of the report this slice
/// fabricates, and the oracle gate declares it as such.
const NO_DERIVED_UNITS: &str = "-";

/// The scope and the key python gives the document, the one element keyed by a fixed name.
const DOCUMENT_KEY: &str = "document";

/// The key python gives a model which carries neither an id nor a metaId (`MAIN_MODEL_KEY`).
const MAIN_MODEL_KEY: &str = "model";

/// The shared fields of an element, keyed in `scope` under `key`.
///
/// It is the one place of the builder which makes an `SBase`, through [`SBase::new`], which takes
/// the type of the element: a report object of no `sbmlType` is one the frontend cannot read.
fn sbase(scope: &str, sbml_type: &'static str, element: &Element, key: &str) -> SBase {
    SBase {
        id: element.id.clone(),
        meta_id: element.meta_id.clone(),
        name: element.name.clone(),
        sbo: element.sbo.clone(),
        notes: element.notes.clone(),
        xml: element.xml.clone(),
        ..SBase::new(pk::of(scope, sbml_type, key), sbml_type)
    }
}

/// The key of an element of a list, its id, else its metaId, else its position in that list.
fn key_in_list(element: &Element, index: usize) -> String {
    pk::key(element, || index.to_string())
}

/// The key of an element whose aliased id names what it sets: an initial assignment or a rule.
///
/// Python reads the `id` attribute of the document first, then the target libsbml aliases to the id
/// of the element (`KEYED_BY_TARGET_CLASSES`, `sbmlinfo.py:457-458`), then the metaId, then the key
/// its parent derived for it. The target comes before the metaId, which is the order of `_key`
/// (`sbmlinfo.py:452-463`) and not the one of [`pk::key`]: a model holds at most one initial
/// assignment per symbol and at most one rule per variable (core §4.8.2, §4.9.1), so the target
/// keys the element, and it keeps the permalink of a rule of a Level 2 document, which can carry no
/// id at all, independent of the metaId.
///
/// `target` is `None` for a target the document does not name, which is every algebraic rule and
/// an assignment of an invalid document, and `fallback` answers the key the parent derived.
fn key_of_target(
    element: &Element,
    target: Option<&str>,
    fallback: impl FnOnce() -> String,
) -> String {
    element
        .id
        .as_deref()
        .or(target)
        .or(element.meta_id.as_deref())
        .map_or_else(fallback, str::to_owned)
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
fn compartment(scope: &str, index: usize, c: &Compartment) -> ReportCompartment {
    ReportCompartment {
        sbase: sbase(
            scope,
            "Compartment",
            &c.element,
            &key_in_list(&c.element, index),
        ),
        spatial_dimensions: c.spatial_dimensions.map(Double),
        size: c.size.map(Double),
        constant: c.constant,
        units: c.units.clone(),
        units_latex: None,
        derived_units: Some(NO_DERIVED_UNITS.to_owned()),
    }
}

/// A species of a model.
fn species(scope: &str, index: usize, s: &Species) -> ReportSpecies {
    ReportSpecies {
        sbase: sbase(
            scope,
            "Species",
            &s.element,
            &key_in_list(&s.element, index),
        ),
        compartment: s.compartment.clone(),
        initial_amount: s.initial_amount.map(Double),
        initial_concentration: s.initial_concentration.map(Double),
        substance_units: s.substance_units.clone(),
        has_only_substance_units: s.has_only_substance_units,
        boundary_condition: s.boundary_condition,
        constant: s.constant,
        units_latex: None,
        derived_units: Some(NO_DERIVED_UNITS.to_owned()),
        conversion_factor: None,
        fbc: None,
    }
}

/// A global parameter of a model.
fn parameter(scope: &str, index: usize, p: &Parameter) -> ReportParameter {
    ReportParameter {
        sbase: sbase(
            scope,
            "Parameter",
            &p.element,
            &key_in_list(&p.element, index),
        ),
        value: p.value.map(Double),
        constant: p.constant,
        units: p.units.clone(),
        units_latex: None,
        derived_units: Some(NO_DERIVED_UNITS.to_owned()),
    }
}

/// A function definition with its lambda.
///
/// # Errors
///
/// What [`math_of`] answers for the math of the definition.
fn function_definition(
    scope: &str,
    index: usize,
    fd: &FunctionDefinition,
) -> Result<ReportFunctionDefinition, BuildError> {
    let sbase = sbase(
        scope,
        "FunctionDefinition",
        &fd.element,
        &key_in_list(&fd.element, index),
    );
    let math = math_of(fd.math.as_ref(), &sbase.pk)?;
    Ok(ReportFunctionDefinition { sbase, math })
}

/// A unit definition with its factors.
fn unit_definition(scope: &str, index: usize, ud: &UnitDefinition) -> ReportUnitDefinition {
    ReportUnitDefinition {
        sbase: sbase(
            scope,
            "UnitDefinition",
            &ud.element,
            &key_in_list(&ud.element, index),
        ),
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
    index: usize,
    ia: &InitialAssignment,
) -> Result<ReportInitialAssignment, BuildError> {
    // the symbol is a required attribute, so the empty string is a document which names none
    let symbol = (!ia.symbol.is_empty()).then_some(ia.symbol.as_str());
    let key = key_of_target(&ia.element, symbol, || index.to_string());
    let sbase = sbase(scope, "InitialAssignment", &ia.element, &key);
    let math = math_of(ia.math.as_ref(), &sbase.pk)?;
    Ok(ReportInitialAssignment {
        sbase,
        symbol: ia.symbol.clone(),
        math,
        derived_units: Some(NO_DERIVED_UNITS.to_owned()),
    })
}

/// A rule of one of the three rule classes of SBML.
///
/// `algebraic` is the number of algebraic rules of the model before this one, which keys an
/// algebraic rule that carries no identifier of its own.
///
/// # Errors
///
/// What [`math_of`] answers for the math of the rule.
fn rule(
    scope: &str,
    index: usize,
    algebraic: usize,
    rule: &Rule,
) -> Result<ReportRule, BuildError> {
    let sbml_type = match rule.kind {
        RuleKind::Assignment => "AssignmentRule",
        RuleKind::Rate => "RateRule",
        RuleKind::Algebraic => "AlgebraicRule",
    };
    let key = key_of_target(&rule.element, rule.variable.as_deref(), || {
        match rule.kind {
            RuleKind::Algebraic => format!("algebraicRule.{algebraic}"),
            RuleKind::Assignment | RuleKind::Rate => index.to_string(),
        }
    });
    let sbase = sbase(scope, sbml_type, &rule.element, &key);
    let math = math_of(rule.math.as_ref(), &sbase.pk)?;
    Ok(ReportRule {
        sbase,
        // the class of the algebraic rule declares no variable, so its object carries no such
        // key; the two others declare a required string, which python writes as the empty
        // string for a document which names none
        variable: match rule.kind {
            RuleKind::Algebraic => None,
            RuleKind::Assignment | RuleKind::Rate => {
                Some(rule.variable.clone().unwrap_or_default())
            }
        },
        math,
        derived_units: Some(NO_DERIVED_UNITS.to_owned()),
    })
}

/// A constraint with its message.
///
/// # Errors
///
/// What [`math_of`] answers for the math of the constraint.
fn constraint(scope: &str, index: usize, c: &Constraint) -> Result<ReportConstraint, BuildError> {
    // a constraint carries nothing which names it, so one without an identifier is keyed by
    // its place in the list of the constraints (`sbmlinfo.py:870`)
    let key = pk::key(&c.element, || format!("constraint.{index}"));
    let sbase = sbase(scope, "Constraint", &c.element, &key);
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

/// A local parameter of a kinetic law, the one element whose own id never keys it.
///
/// Python passes `use_id=False` (`sbmlinfo.py:1186-1203`): the id of a local parameter is a genuine
/// id, required by SBML, but scoped to its own kinetic law and not to the model, since two laws may
/// each hold a parameter of the same id, so the key carries the law instead. Its metaId, which is
/// unique within the document, still wins over that key (`_key`, `sbmlinfo.py:447-452`), which is
/// why this key is built here and reaches for neither [`pk::key`] nor [`pk::derived`]. The `id`
/// field of the report object carries the id of the parameter all the same.
///
/// `law_key` is the key the reaction derived for its law and not the key the law is reported under,
/// which is what python builds the key of a local parameter from: a law with a metaId of its own is
/// keyed by it and its parameters stay keyed by the reaction.
fn local_parameter(scope: &str, law_key: &str, lp: &LocalParameter) -> ReportLocalParameter {
    let key = lp.element.meta_id.clone().unwrap_or_else(|| {
        // python reads the id of the parameter unguarded, so a parameter of an invalid document
        // which carries none is keyed by the law and the empty string, as python keys it
        format!("{law_key}.{}", lp.element.id.as_deref().unwrap_or_default())
    });
    ReportLocalParameter {
        sbase: sbase(scope, "LocalParameter", &lp.element, &key),
        value: lp.value.map(Double),
        units: lp.units.clone(),
        units_latex: None,
        derived_units: Some(NO_DERIVED_UNITS.to_owned()),
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
        derived_units: Some(NO_DERIVED_UNITS.to_owned()),
        list_of_local_parameters: law
            .local_parameters
            .iter()
            .map(|lp| local_parameter(scope, &derived, lp))
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
    index: usize,
    level: u32,
    version: u32,
    r: &Reaction,
) -> Result<ReportReaction, BuildError> {
    let key = key_in_list(&r.element, index);
    let law = r
        .kinetic_law
        .as_ref()
        .map(|law| kinetic_law(scope, &key, law))
        .transpose()?;
    Ok(ReportReaction {
        sbase: sbase(scope, "Reaction", &r.element, &key),
        reversible: reversible(r, level),
        fast: fast(r, level, version),
        compartment: r.compartment.clone(),
        list_of_reactants: species_references(scope, &key, "reactant", &r.reactants, level),
        list_of_products: species_references(scope, &key, "product", &r.products, level),
        list_of_modifiers: modifier_species_references(scope, &key, &r.modifiers),
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
/// # Errors
///
/// What [`math_of`] answers for the math of the first element of the event whose math no writer
/// writes.
fn event(scope: &str, index: usize, e: &Event) -> Result<ReportEvent, BuildError> {
    // an event carries nothing which names it, so one without an identifier is keyed by its place
    // in the list of the events (`sbmlinfo.py:876-879`)
    let key = pk::key(&e.element, || format!("event.{index}"));
    Ok(ReportEvent {
        sbase: sbase(scope, "Event", &e.element, &key),
        use_values_from_trigger_time: e.use_values_from_trigger_time,
        trigger: e
            .trigger
            .as_ref()
            .map(|t| trigger(scope, &key, t))
            .transpose()?,
        priority: e
            .priority
            .as_ref()
            .map(|p| priority(scope, &key, p))
            .transpose()?,
        delay: e
            .delay
            .as_ref()
            .map(|d| delay(scope, &key, d))
            .transpose()?,
        list_of_event_assignments: event_assignments(scope, &key, &e.event_assignments)?,
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
fn model(model: &Model, level: u32, version: u32) -> Result<ReportModel, BuildError> {
    // the key of the model is the scope of every element below it
    let scope = pk::key(&model.element, || MAIN_MODEL_KEY.to_owned());
    // an algebraic rule sets no variable, so one which carries no identifier is keyed by its place
    // among the algebraic rules alone, which an edit of another rule does not move; the place is
    // counted for every algebraic rule, as `rules` of `sbmlinfo.py:1031-1049` counts it, so that
    // one which is keyed by an identifier of its own does not move the key of a later one
    let mut algebraic: usize = 0;
    let mut rules = Vec::with_capacity(model.rules.len());
    for (i, r) in model.rules.iter().enumerate() {
        rules.push(rule(&scope, i, algebraic, r)?);
        if matches!(r.kind, RuleKind::Algebraic) {
            algebraic += 1;
        }
    }
    Ok(ReportModel {
        // the model is keyed by the key which is its own scope, python writes `<key>/Model:<key>`
        sbase: sbase(&scope, "Model", &model.element, &scope),
        kind: "model",
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
            .enumerate()
            .map(|(i, fd)| function_definition(&scope, i, fd))
            .collect::<Result<Vec<_>, BuildError>>()?,
        list_of_unit_definitions: model
            .unit_definitions
            .iter()
            .enumerate()
            .map(|(i, ud)| unit_definition(&scope, i, ud))
            .collect(),
        list_of_compartments: model
            .compartments
            .iter()
            .enumerate()
            .map(|(i, c)| compartment(&scope, i, c))
            .collect(),
        list_of_species: model
            .species
            .iter()
            .enumerate()
            .map(|(i, s)| species(&scope, i, s))
            .collect(),
        list_of_parameters: model
            .parameters
            .iter()
            .enumerate()
            .map(|(i, p)| parameter(&scope, i, p))
            .collect(),
        list_of_initial_assignments: model
            .initial_assignments
            .iter()
            .enumerate()
            .map(|(i, ia)| initial_assignment(&scope, i, ia))
            .collect::<Result<Vec<_>, BuildError>>()?,
        list_of_rules: rules,
        list_of_constraints: model
            .constraints
            .iter()
            .enumerate()
            .map(|(i, c)| constraint(&scope, i, c))
            .collect::<Result<Vec<_>, BuildError>>()?,
        list_of_reactions: model
            .reactions
            .iter()
            .enumerate()
            .map(|(i, r)| reaction(&scope, i, level, version, r))
            .collect::<Result<Vec<_>, BuildError>>()?,
        list_of_events: model
            .events
            .iter()
            .enumerate()
            .map(|(i, e)| event(&scope, i, e))
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
    Ok(Report {
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
            .model
            .iter()
            .map(|m| model(m, document.level, document.version))
            .collect::<Result<Vec<_>, BuildError>>()?,
        external_model_definitions: vec![],
        link_graph: LinkGraph::default(),
        ode_system: None,
        ode_error: None,
    })
}
