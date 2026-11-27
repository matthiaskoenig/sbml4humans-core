//! The link graph of the supported report objects, without XML or formula reparsing.
//!
//! Keys come from the finished report and math references from the owned AST. Keeping
//! those inputs separate preserves both identifier namespaces and the owner of each math.

mod extension;

use std::collections::{BTreeSet, HashMap, HashSet};

use sbml4humans_model::{Ast, AstKind, Document};

use crate::{EdgeKind, LinkEdge, LinkGraph, LinkNode, Report, ReportModel, SBase};

/// The references of every math with this owner, merged when malformed objects share a key.
#[derive(Default)]
struct References {
    symbols: BTreeSet<String>,
    units: BTreeSet<String>,
}

type Maths = HashMap<String, References>;

fn add_math(maths: &mut Maths, owner: &SBase, ast: Option<&Ast>) {
    let Some(ast) = ast else { return };
    let refs = maths.entry(owner.pk.clone()).or_default();
    refs.symbols.extend(crate::math::free_symbols(ast));
    // Units are collected independently: lambda binder subtrees do not reference names,
    // but the Python report still visits their numbers when collecting units.
    let mut stack = vec![ast];
    while let Some(node) = stack.pop() {
        if matches!(
            node.kind,
            AstKind::Integer | AstKind::Real | AstKind::RealE | AstKind::Rational
        ) && let Some(units) = &node.units
        {
            refs.units.insert(units.clone());
        }
        stack.extend(&node.children);
    }
}

fn maths(report: &Report, document: &Document) -> Maths {
    let mut maths = Maths::new();
    macro_rules! collect {
        ($input:expr, $output:expr) => {
            for (input, output) in ($input).iter().zip(($output).iter()) {
                add_math(&mut maths, &output.sbase, input.math.as_ref());
            }
        };
    }
    for (input, output) in document.models().zip(&report.models) {
        collect!(
            &input.function_definitions,
            &output.list_of_function_definitions
        );
        collect!(
            &input.initial_assignments,
            &output.list_of_initial_assignments
        );
        collect!(&input.rules, &output.list_of_rules);
        collect!(&input.constraints, &output.list_of_constraints);
        for (input, output) in input.reactions.iter().zip(&output.list_of_reactions) {
            collect!(&input.kinetic_law, &output.kinetic_law);
        }
        for (input, output) in input.events.iter().zip(&output.list_of_events) {
            collect!(&input.trigger, &output.trigger);
            collect!(&input.priority, &output.priority);
            collect!(&input.delay, &output.delay);
            collect!(&input.event_assignments, &output.list_of_event_assignments);
        }
    }
    maths
}

/// Objects in the global SId namespace, in the same precedence order as Python's ModelIndex.
fn elements(model: &ReportModel) -> Vec<&SBase> {
    let mut elements = Vec::new();
    macro_rules! add {
        ($list:expr) => {
            elements.extend(($list).iter().map(|e| &e.sbase));
        };
    }
    add!(&model.list_of_function_definitions);
    add!(&model.list_of_compartments);
    add!(&model.list_of_species);
    add!(&model.list_of_parameters);
    add!(&model.list_of_initial_assignments);
    add!(&model.list_of_rules);
    add!(&model.list_of_constraints);
    for reaction in &model.list_of_reactions {
        elements.push(&reaction.sbase);
        add!(&reaction.list_of_reactants);
        add!(&reaction.list_of_products);
        add!(&reaction.list_of_modifiers);
        add!(&reaction.kinetic_law);
    }
    for event in &model.list_of_events {
        elements.push(&event.sbase);
        add!(&event.trigger);
        add!(&event.priority);
        add!(&event.delay);
        add!(&event.list_of_event_assignments);
    }
    elements
}

struct ModelIndex<'a> {
    sids: HashMap<&'a str, &'a str>,
    units: HashMap<&'a str, &'a str>,
    locals: HashMap<&'a str, HashMap<&'a str, &'a str>>,
}

impl<'a> ModelIndex<'a> {
    fn new(model: &'a ReportModel) -> Self {
        let mut sids = HashMap::new();
        for element in elements(model) {
            if let Some(id) = element.id.as_deref() {
                // The first global object of a duplicate id wins; units and local
                // parameters have separate namespaces with last-definition precedence.
                sids.entry(id).or_insert(element.pk.as_str());
            }
        }
        for object in extension::global(model) {
            if let Some(id) = object.id() {
                sids.entry(id).or_insert(object.pk());
            }
        }
        for root in extension::roots(model) {
            for object in extension::with_extensions(root) {
                if matches!(
                    object.type_(),
                    "Uncertainty" | "UncertParameter" | "UncertSpan" | "ListOf"
                ) && let Some(id) = object.id()
                {
                    sids.entry(id).or_insert(object.pk());
                }
            }
        }
        let units = model
            .list_of_unit_definitions
            .iter()
            .filter_map(|u| u.sbase.id.as_deref().map(|id| (id, u.sbase.pk.as_str())))
            .collect();
        let locals = model
            .list_of_reactions
            .iter()
            .filter_map(|r| r.kinetic_law.as_ref())
            .map(|law| {
                (
                    law.sbase.pk.as_str(),
                    law.list_of_local_parameters
                        .iter()
                        .filter_map(|p| p.sbase.id.as_deref().map(|id| (id, p.sbase.pk.as_str())))
                        .collect(),
                )
            })
            .collect();
        Self {
            sids,
            units,
            locals,
        }
    }

    fn resolve(&self, sid: &str, law: Option<&str>) -> Option<&'a str> {
        law.and_then(|law| self.locals.get(law))
            .and_then(|locals| locals.get(sid))
            .or_else(|| self.sids.get(sid))
            .copied()
    }
}

#[derive(Default)]
struct Builder {
    graph: LinkGraph,
    // Node positions identify edges without retaining a second copy of every key.
    seen: HashSet<(usize, usize, EdgeKind)>,
}

impl Builder {
    fn edge(&mut self, source: &SBase, target: &str, kind: EdgeKind) {
        self.edge_pk(&source.pk, target, kind);
    }

    fn edge_pk(&mut self, source: &str, target: &str, kind: EdgeKind) {
        let Some(source_index) = self.graph.nodes.get_index_of(source) else {
            return;
        };
        let Some(target_index) = self.graph.nodes.get_index_of(target) else {
            return;
        };
        if self.seen.insert((source_index, target_index, kind)) {
            self.graph.edges.push(LinkEdge {
                source: source.to_owned(),
                target: target.to_owned(),
                kind,
                target_entry: None,
            });
        }
    }

    fn reference(&mut self, source: &SBase, sid: Option<&str>, kind: EdgeKind, index: &ModelIndex) {
        if let Some(target) = sid.and_then(|sid| index.resolve(sid, None)) {
            self.edge(source, target, kind);
        }
    }

    fn units(&mut self, source: &SBase, units: Option<&str>, index: &ModelIndex) {
        if let Some(target) = units.and_then(|sid| index.units.get(sid)) {
            self.edge(source, target, EdgeKind::Units);
        }
    }

    fn math(&mut self, owner: &SBase, maths: &Maths, index: &ModelIndex, law: Option<&str>) {
        if let Some(refs) = maths.get(&owner.pk) {
            for symbol in &refs.symbols {
                if let Some(target) = index.resolve(symbol, law) {
                    self.edge(owner, target, EdgeKind::Math);
                }
            }
            for units in &refs.units {
                self.units(owner, Some(units), index);
            }
        }
    }

    fn model(&mut self, model: &ReportModel, level: i64, maths: &Maths, index: &ModelIndex) {
        for units in [
            &model.substance_units,
            &model.time_units,
            &model.volume_units,
            &model.area_units,
            &model.length_units,
            &model.extent_units,
        ] {
            self.units(&model.sbase, units.as_deref(), index);
        }
        if level < 3 {
            for units in ["substance", "time", "volume", "area", "length"] {
                self.units(&model.sbase, Some(units), index);
            }
        }
        if let Some(factor) = &model.conversion_factor {
            self.reference(
                &model.sbase,
                Some(&factor.sid),
                EdgeKind::ConversionFactor,
                index,
            );
        }
        for f in &model.list_of_function_definitions {
            self.math(&f.sbase, maths, index, None);
        }
        for c in &model.list_of_compartments {
            self.units(&c.sbase, c.units.as_deref(), index);
        }
        for s in &model.list_of_species {
            self.reference(&s.sbase, Some(&s.compartment), EdgeKind::Compartment, index);
            self.units(&s.sbase, s.substance_units.as_deref(), index);
            if let Some(factor) = &s.conversion_factor {
                self.reference(
                    &s.sbase,
                    Some(&factor.sid),
                    EdgeKind::ConversionFactor,
                    index,
                );
            }
        }
        for p in &model.list_of_parameters {
            self.units(&p.sbase, p.units.as_deref(), index);
        }
        for a in &model.list_of_initial_assignments {
            self.reference(&a.sbase, Some(&a.symbol), EdgeKind::Symbol, index);
            self.math(&a.sbase, maths, index, None);
        }
        for rule in &model.list_of_rules {
            self.reference(
                &rule.sbase,
                rule.variable.as_deref(),
                EdgeKind::Variable,
                index,
            );
            self.math(&rule.sbase, maths, index, None);
        }
        for c in &model.list_of_constraints {
            self.math(&c.sbase, maths, index, None);
        }
        for r in &model.list_of_reactions {
            self.reference(
                &r.sbase,
                r.compartment.as_deref(),
                EdgeKind::Compartment,
                index,
            );
            for (base, species, kind) in r
                .list_of_reactants
                .iter()
                .map(|s| (&s.sbase, s.species.as_str(), EdgeKind::Reactant))
                .chain(
                    r.list_of_products
                        .iter()
                        .map(|s| (&s.sbase, s.species.as_str(), EdgeKind::Product)),
                )
                .chain(
                    r.list_of_modifiers
                        .iter()
                        .map(|s| (&s.sbase, s.species.as_str(), EdgeKind::Modifier)),
                )
            {
                self.edge(&r.sbase, &base.pk, kind);
                self.reference(base, Some(species), kind, index);
            }
            extension::reaction_edges(self, r, index);
            if let Some(law) = &r.kinetic_law {
                self.edge(&r.sbase, &law.sbase.pk, EdgeKind::KineticLaw);
                self.math(&law.sbase, maths, index, Some(&law.sbase.pk));
                for p in &law.list_of_local_parameters {
                    self.edge(&law.sbase, &p.sbase.pk, EdgeKind::LocalParameter);
                    self.units(&p.sbase, p.units.as_deref(), index);
                }
            }
        }
        for event in &model.list_of_events {
            let children = event
                .trigger
                .iter()
                .map(|c| (&c.sbase, EdgeKind::Trigger))
                .chain(
                    event
                        .priority
                        .iter()
                        .map(|c| (&c.sbase, EdgeKind::Priority)),
                )
                .chain(event.delay.iter().map(|c| (&c.sbase, EdgeKind::Delay)));
            let children: Vec<_> = children.collect();
            for &(child, kind) in &children {
                self.edge(&event.sbase, &child.pk, kind);
            }
            for &(child, _) in &children {
                self.math(child, maths, index, None);
            }
            for assignment in &event.list_of_event_assignments {
                self.edge(
                    &event.sbase,
                    &assignment.sbase.pk,
                    EdgeKind::EventAssignment,
                );
                self.reference(
                    &assignment.sbase,
                    Some(&assignment.variable),
                    EdgeKind::Variable,
                    index,
                );
                self.math(&assignment.sbase, maths, index, None);
            }
        }
    }
}

pub(crate) fn build(report: &Report, document: &Document) -> LinkGraph {
    let mut maths = maths(report, document);
    extension::package_maths(report, document, &mut maths);
    let mut builder = Builder::default();
    extension::nodes(&mut builder, report);
    extension::document_edges(&mut builder, report);
    for model in &report.models {
        builder.edge(&report.document.sbase, &model.sbase.pk, EdgeKind::Model);
    }
    for emd in &report.external_model_definitions {
        if let Some(pk) = emd.get("pk").and_then(serde_json::Value::as_str) {
            builder.edge(
                &report.document.sbase,
                pk,
                EdgeKind::ExternalModelDefinition,
            );
        }
    }
    let universe = extension::Universe::new(report);
    for (i, model) in report.models.iter().enumerate() {
        extension::metadata_edges(&mut builder, &universe, model, i, &maths);
        builder.model(model, report.document.level, &maths, universe.index(i));
        extension::model_edges(&mut builder, &universe, model, i, &maths);
    }
    builder.graph
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbml4humans_model::{AstNumber, Element, Model, Parameter, Species, UnitDefinition};

    fn ast(kind: AstKind, name: Option<&str>, children: Vec<Ast>) -> Ast {
        Ast {
            kind,
            name: name.map(str::to_owned),
            number: None,
            units: None,
            children,
        }
    }

    fn name(value: &str) -> Ast {
        ast(AstKind::Name, Some(value), vec![])
    }

    fn element(id: &str) -> Element {
        Element {
            id: Some(id.to_owned()),
            ..Element::default()
        }
    }

    #[test]
    fn lambda_bindings_are_scoped_and_user_functions_are_references() {
        let inner = ast(AstKind::Lambda, None, vec![name("x"), name("x")]);
        let outer = ast(
            AstKind::Lambda,
            None,
            vec![
                name("x"),
                ast(
                    AstKind::Plus,
                    None,
                    vec![
                        inner,
                        ast(AstKind::Function, Some("f"), vec![name("x"), name("g")]),
                        ast(AstKind::FunctionSin, Some("sin"), vec![name("x")]),
                    ],
                ),
            ],
        );
        let root = ast(AstKind::Plus, None, vec![outer, name("x")]);
        let mut found = BTreeSet::new();
        found.extend(crate::math::free_symbols(&root));
        assert_eq!(found, BTreeSet::from(["f".into(), "g".into(), "x".into()]));
    }

    #[test]
    fn units_include_numbers_in_lambda_binders_but_not_units_on_names() {
        let mut number = ast(AstKind::Integer, None, vec![]);
        number.number = Some(AstNumber::Integer(1));
        number.units = Some("u".into());
        let mut variable = name("x");
        variable.units = Some("not_a_number".into());
        let root = ast(AstKind::Lambda, None, vec![number, variable]);
        let mut maths = Maths::new();
        let owner = SBase::new("m/FunctionDefinition:f".into(), "FunctionDefinition");
        add_math(&mut maths, &owner, Some(&root));
        assert_eq!(maths[&owner.pk].units, BTreeSet::from(["u".into()]));
    }

    #[test]
    fn very_deep_binding_walk_and_drop_do_not_need_a_call_frame_per_level() {
        let mut root = name("outside");
        for _ in 0..sbml4humans_model::MAX_AST_DEPTH {
            root = ast(AstKind::Lambda, None, vec![name("x"), root]);
        }
        let mut found = BTreeSet::new();
        found.extend(crate::math::free_symbols(&root));
        assert_eq!(found, BTreeSet::from(["outside".into()]));
    }

    #[test]
    fn duplicate_math_owners_merge_references_as_the_python_report_does() {
        let mut maths = Maths::new();
        let owner = SBase::new("m/InitialAssignment:a".into(), "InitialAssignment");
        add_math(&mut maths, &owner, Some(&name("z")));
        add_math(&mut maths, &owner, Some(&name("a")));
        add_math(&mut maths, &owner, Some(&name("z")));
        assert_eq!(
            maths[&owner.pk].symbols,
            BTreeSet::from(["a".into(), "z".into()])
        );
    }

    fn report_with_namespaces() -> Report {
        let mut input = Model {
            element: element("m"),
            unit_definitions: vec![UnitDefinition {
                element: element("shared"),
                ..Default::default()
            }],
            compartments: vec![sbml4humans_model::Compartment {
                element: element("shared"),
                ..Default::default()
            }],
            species: vec![Species {
                element: element("S"),
                compartment: "shared".into(),
                ..Default::default()
            }],
            parameters: vec![Parameter {
                element: element("shared"),
                ..Default::default()
            }],
            ..Default::default()
        };
        input.reactions.push(sbml4humans_model::Reaction {
            element: element("r"),
            kinetic_law: Some(sbml4humans_model::KineticLaw {
                local_parameters: vec![sbml4humans_model::LocalParameter {
                    element: element("shared"),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        });
        crate::build(&Document {
            level: 3,
            version: 2,
            model: Some(input),
            ..Default::default()
        })
        .unwrap()
    }

    #[test]
    fn global_unit_and_local_namespaces_keep_their_own_precedence() {
        let report = report_with_namespaces();
        let model = &report.models[0];
        let law = model.list_of_reactions[0].kinetic_law.as_ref().unwrap();
        let index = ModelIndex::new(model);
        assert_eq!(index.resolve("shared", None), Some("m/Compartment:shared"));
        assert_eq!(index.units["shared"], "m/UnitDefinition:shared");
        assert_eq!(
            index.resolve("shared", Some(&law.sbase.pk)),
            Some(law.list_of_local_parameters[0].sbase.pk.as_str())
        );
        assert_eq!(index.resolve("missing", Some(&law.sbase.pk)), None);
    }

    #[test]
    fn graph_nodes_keep_document_traversal_order_and_model_ownership() {
        let report = report_with_namespaces();
        let model = &report.models[0];
        let law = model.list_of_reactions[0].kinetic_law.as_ref().unwrap();
        let keys: Vec<_> = report.link_graph.nodes.keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            [
                "document/SBMLDocument:document",
                "m/Model:m",
                "m/UnitDefinition:shared",
                "m/Compartment:shared",
                "m/Species:S",
                "m/Parameter:shared",
                "m/Reaction:r",
                law.sbase.pk.as_str(),
                law.list_of_local_parameters[0].sbase.pk.as_str(),
            ]
        );
        assert!(report.link_graph.nodes["m/Model:m"].model.is_none());
        assert_eq!(
            report.link_graph.nodes["m/Species:S"].model.as_deref(),
            Some("m/Model:m")
        );
        // Verify serialization itself, since a sorted JSON Value would discard map order.
        let json = serde_json::to_string(&report.link_graph).unwrap();
        assert!(
            json.find("m/UnitDefinition:shared").unwrap()
                < json.find("m/Compartment:shared").unwrap()
        );
    }

    #[test]
    fn edge_deduplication_keeps_first_order_and_distinct_roles_and_self_references() {
        let report = report_with_namespaces();
        let source = &report.models[0].list_of_species[0].sbase;
        let mut builder = Builder {
            graph: report.link_graph,
            ..Default::default()
        };
        builder.graph.edges.clear();
        builder.edge(source, &source.pk, EdgeKind::Math);
        builder.edge(source, "m/Compartment:shared", EdgeKind::Compartment);
        builder.edge(source, &source.pk, EdgeKind::Math);
        builder.edge(source, &source.pk, EdgeKind::Variable);
        assert_eq!(builder.graph.edges.len(), 3);
        assert_eq!(builder.graph.edges[0].kind, EdgeKind::Math);
        assert_eq!(builder.graph.edges[1].kind, EdgeKind::Compartment);
        assert_eq!(builder.graph.edges[2].kind, EdgeKind::Variable);
        assert_eq!(builder.graph.edges[0].source, builder.graph.edges[0].target);
        assert!(builder.graph.edges.iter().all(|e| e.target_entry.is_none()));
    }
}
