//! Borrowed graph views of core objects and package projections.
//! No second JSON report, XML parse or formula parse is required.
use super::*;
use crate::{ListOf, ReportReaction, Uncertainty};
use serde_json::Value;

#[derive(Clone, Copy)]
pub(super) enum View<'a> {
    Core(&'a SBase),
    Json(&'a Value),
    Uncertainty(&'a Uncertainty),
    List(&'a ListOf),
}
impl<'a> View<'a> {
    fn base(self) -> Option<&'a SBase> {
        match self {
            Self::Core(b) => Some(b),
            Self::Uncertainty(u) => Some(&u.sbase),
            Self::List(l) => Some(&l.sbase),
            _ => None,
        }
    }
    fn string(self, field: &str) -> Option<&'a str> {
        if let Some(b) = self.base() {
            match field {
                "pk" => Some(&b.pk),
                "sbmlType" => Some(b.sbml_type),
                "id" => b.id.as_deref(),
                "metaId" => b.meta_id.as_deref(),
                "name" => b.name.as_deref(),
                "xml" => b.xml.as_deref(),
                _ => None,
            }
        } else if let Self::Json(v) = self {
            v.get(field).and_then(Value::as_str)
        } else {
            None
        }
    }
    pub(super) fn pk(self) -> &'a str {
        self.string("pk").unwrap_or("")
    }
    pub(super) fn id(self) -> Option<&'a str> {
        self.string("id")
    }
    pub(super) fn type_(self) -> &'a str {
        self.string("sbmlType").unwrap_or("")
    }
    fn one(self, field: &str) -> Option<View<'a>> {
        if let Self::Json(v) = self {
            v.get(field).filter(|v| v.is_object()).map(View::Json)
        } else {
            None
        }
    }
    fn children(self, field: &str) -> Vec<View<'a>> {
        if let Some(b) = self.base() {
            match field {
                "uncertainties" => return b.uncertainties.iter().map(View::Uncertainty).collect(),
                "lists" => return b.lists.iter().map(View::List).collect(),
                _ => {}
            }
        }
        match self {
            Self::Uncertainty(u) if field == "uncertParameters" => {
                u.uncert_parameters.iter().map(View::Json).collect()
            }
            Self::Json(v) => v
                .get(field)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(View::Json)
                .collect(),
            _ => vec![],
        }
    }
    fn replacements(self) -> Vec<(View<'a>, EdgeKind)> {
        if let Some(base) = self.base() {
            base.comp
                .iter()
                .flat_map(|c| {
                    c.replaced_by
                        .iter()
                        .map(|v| (View::Json(v), EdgeKind::ReplacedBy))
                        .chain(
                            c.replaced_elements
                                .iter()
                                .map(|v| (View::Json(v), EdgeKind::ReplacedElement)),
                        )
                })
                .collect()
        } else {
            self.one("comp")
                .map(|c| {
                    c.one("replacedBy")
                        .map(|v| (v, EdgeKind::ReplacedBy))
                        .into_iter()
                        .chain(
                            c.children("replacedElements")
                                .into_iter()
                                .map(|v| (v, EdgeKind::ReplacedElement)),
                        )
                        .collect()
                })
                .unwrap_or_default()
        }
    }
}
fn append<'a>(out: &mut Vec<View<'a>>, objects: &'a [Value]) {
    out.extend(objects.iter().map(View::Json));
}
fn association<'a>(root: View<'a>, out: &mut Vec<View<'a>>) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        out.push(node);
        let mut children = node.children("associations");
        if let Some(child) = node.one("association") {
            children.push(child);
        }
        stack.extend(children.into_iter().rev());
    }
}
pub(super) fn global(model: &ReportModel) -> Vec<View<'_>> {
    let mut out = Vec::new();
    macro_rules! add {
        ($list:expr) => {
            out.extend(($list).iter().map(|e| View::Core(&e.sbase)));
        };
    }
    add!(&model.list_of_function_definitions);
    add!(&model.list_of_compartments);
    add!(&model.list_of_species);
    add!(&model.list_of_parameters);
    add!(&model.list_of_initial_assignments);
    add!(&model.list_of_rules);
    add!(&model.list_of_constraints);
    for r in &model.list_of_reactions {
        out.push(View::Core(&r.sbase));
        add!(&r.list_of_reactants);
        add!(&r.list_of_products);
        add!(&r.list_of_modifiers);
        add!(&r.kinetic_law);
        if let Some(gpa) = r
            .fbc
            .as_ref()
            .and_then(|f| f.get("geneProductAssociation"))
            .filter(|g| g.is_object())
        {
            association(View::Json(gpa), &mut out);
        }
    }
    for e in &model.list_of_events {
        out.push(View::Core(&e.sbase));
        add!(&e.trigger);
        add!(&e.priority);
        add!(&e.delay);
        add!(&e.list_of_event_assignments);
    }
    append(&mut out, &model.list_of_submodels);
    append(&mut out, &model.list_of_gene_products);
    append(&mut out, &model.list_of_flux_bounds);
    for c in &model.list_of_user_defined_constraints {
        let v = View::Json(c);
        out.push(v);
        out.extend(v.children("listOfUserDefinedConstraintComponents"));
    }
    for o in &model.list_of_objectives {
        let v = View::Json(o);
        out.push(v);
        out.extend(v.children("listOfFluxObjectives"));
    }
    append(&mut out, &model.list_of_qualitative_species);
    for t in &model.list_of_transitions {
        let v = View::Json(t);
        out.push(v);
        out.extend(v.children("listOfInputs"));
        out.extend(v.children("listOfOutputs"));
    }
    out
}
fn chain<'a>(mut node: View<'a>, out: &mut Vec<View<'a>>) {
    loop {
        out.push(node);
        if let Some(next) = node.one("sbaseRef") {
            node = next;
        } else {
            break;
        }
    }
}
pub(super) fn roots(model: &ReportModel) -> Vec<View<'_>> {
    let mut out = vec![View::Core(&model.sbase)];
    out.extend(
        model
            .list_of_unit_definitions
            .iter()
            .map(|e| View::Core(&e.sbase)),
    );
    out.extend(global(model));
    append(&mut out, &model.list_of_ports);
    for r in &model.list_of_reactions {
        if let Some(law) = &r.kinetic_law {
            out.extend(
                law.list_of_local_parameters
                    .iter()
                    .map(|p| View::Core(&p.sbase)),
            );
        }
    }
    for t in &model.list_of_transitions {
        let t = View::Json(t);
        out.extend(t.children("listOfFunctionTerms"));
        out.extend(t.one("defaultTerm"));
    }
    for s in &model.list_of_submodels {
        for d in View::Json(s).children("listOfDeletions") {
            chain(d, &mut out);
        }
    }
    for p in &model.list_of_ports {
        if let Some(r) = View::Json(p).one("sbaseRef") {
            chain(r, &mut out);
        }
    }
    out
}
fn measures<'a>(node: View<'a>, out: &mut Vec<View<'a>>) {
    let mut stack = node.children("uncertParameters");
    stack.reverse();
    while let Some(child) = stack.pop() {
        out.push(child);
        stack.extend(child.children("uncertParameters").into_iter().rev());
    }
}
pub(super) fn with_extensions(root: View<'_>) -> Vec<View<'_>> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        out.push(node);
        let mut children = Vec::new();
        for u in node.children("uncertainties") {
            children.push(u);
            measures(u, &mut children);
        }
        for (r, _) in node.replacements() {
            chain(r, &mut children);
        }
        children.extend(node.children("lists"));
        stack.extend(children.into_iter().rev());
    }
    out
}
fn add_node(builder: &mut Builder, object: View, model: Option<&str>) {
    let pk = object.pk();
    builder.graph.nodes.insert(
        pk.to_owned(),
        LinkNode {
            pk: pk.to_owned(),
            sbml_type: object.type_().to_owned(),
            id: object.id().map(str::to_owned),
            name: object.string("name").map(str::to_owned),
            model: model.filter(|m| *m != pk).map(str::to_owned),
        },
    );
}
pub(super) fn nodes(builder: &mut Builder, report: &Report) {
    for node in with_extensions(View::Core(&report.document.sbase)) {
        add_node(builder, node, None);
    }
    for emd in &report.external_model_definitions {
        for node in with_extensions(View::Json(emd)) {
            add_node(builder, node, None);
        }
    }
    for model in &report.models {
        for root in roots(model) {
            for node in with_extensions(root) {
                add_node(builder, node, Some(&model.sbase.pk));
            }
        }
    }
}
fn direct_edge(b: &mut Builder, source: View, target: View, kind: EdgeKind) {
    b.edge_pk(source.pk(), target.pk(), kind);
}
fn reference(b: &mut Builder, source: View, field: &str, kind: EdgeKind, index: &ModelIndex) {
    if let Some(target) = source.string(field).and_then(|s| index.resolve(s, None)) {
        b.edge_pk(source.pk(), target, kind);
    }
}
fn units(b: &mut Builder, source: View, index: &ModelIndex) {
    if let Some(target) = source.string("units").and_then(|s| index.units.get(s)) {
        b.edge_pk(source.pk(), target, EdgeKind::Units);
    }
}
fn math(b: &mut Builder, source: View, maths: &Maths, index: &ModelIndex) {
    if let Some(refs) = maths.get(source.pk()) {
        for symbol in &refs.symbols {
            if let Some(target) = index.resolve(symbol, None) {
                b.edge_pk(source.pk(), target, EdgeKind::Math);
            }
        }
        for unit in &refs.units {
            if let Some(target) = index.units.get(unit.as_str()) {
                b.edge_pk(source.pk(), target, EdgeKind::Units);
            }
        }
    }
}
fn list_edges(b: &mut Builder, source: View) {
    for child in source.children("lists") {
        direct_edge(b, source, child, EdgeKind::ListOf);
    }
}
pub(super) fn document_edges(b: &mut Builder, report: &Report) {
    for root in std::iter::once(View::Core(&report.document.sbase))
        .chain(report.external_model_definitions.iter().map(View::Json))
    {
        for node in with_extensions(root) {
            list_edges(b, node);
        }
    }
}
struct Index<'a> {
    base: ModelIndex<'a>,
    ports: HashMap<&'a str, View<'a>>,
    meta: HashMap<&'a str, &'a str>,
    submodels: HashMap<&'a str, View<'a>>,
}
pub(super) struct Universe<'a> {
    indices: Vec<Index<'a>>,
    models: HashMap<&'a str, usize>,
    model_refs: HashMap<&'a str, &'a str>,
}
impl<'a> Universe<'a> {
    pub(super) fn new(report: &'a Report) -> Self {
        let mut model_refs: HashMap<_, _> = report
            .external_model_definitions
            .iter()
            .filter_map(|e| Some((e["id"].as_str()?, e["pk"].as_str()?)))
            .collect();
        let mut indices = Vec::new();
        let mut models = HashMap::new();
        for (i, model) in report.models.iter().enumerate() {
            if let Some(id) = model.sbase.id.as_deref() {
                models.insert(id, i);
                model_refs.insert(id, model.sbase.pk.as_str());
            }
            let ports = model
                .list_of_ports
                .iter()
                .map(View::Json)
                .filter_map(|p| p.id().map(|id| (id, p)))
                .collect();
            let mut meta = HashMap::new();
            for root in roots(model) {
                for node in with_extensions(root) {
                    if node.pk() != model.sbase.pk
                        && let Some(id) = node.string("metaId")
                    {
                        meta.entry(id).or_insert(node.pk());
                    }
                }
            }
            let submodels = model
                .list_of_submodels
                .iter()
                .map(View::Json)
                .map(|s| (s.pk(), s))
                .collect();
            indices.push(Index {
                base: ModelIndex::new(model),
                ports,
                meta,
                submodels,
            });
        }
        Self {
            indices,
            models,
            model_refs,
        }
    }
    pub(super) fn index(&self, i: usize) -> &ModelIndex<'a> {
        &self.indices[i].base
    }
    fn model_of(&self, submodel: View) -> Option<usize> {
        submodel
            .string("modelRef")
            .and_then(|id| self.models.get(id))
            .copied()
    }
    fn resolve(
        &self,
        reference: View,
        index: usize,
        seen: &mut HashSet<String>,
        depth: usize,
    ) -> Option<(&'a str, usize)> {
        if depth > 256 {
            return None;
        }
        let current = &self.indices[index];
        let target = if let Some(port) = reference.string("portRef") {
            let port = *current.ports.get(port)?;
            if !seen.insert(port.pk().to_owned()) {
                return None;
            }
            let target = self.resolve(port, index, seen, depth + 1)?;
            if let Some(nested) = port.one("sbaseRef") {
                let submodel = *self.indices[target.1].submodels.get(target.0)?;
                self.resolve(nested, self.model_of(submodel)?, seen, depth + 1)?
            } else {
                target
            }
        } else if let Some(id) = reference.string("idRef") {
            (current.base.resolve(id, None)?, index)
        } else if let Some(id) = reference.string("unitRef") {
            (*current.base.units.get(id)?, index)
        } else {
            let id = reference.string("metaIdRef")?;
            (*current.meta.get(id)?, index)
        };
        Some(target)
    }
    fn resolve_into(&self, reference: View, submodel: View) -> Option<&'a str> {
        let mut current = reference;
        let mut index = self.model_of(submodel)?;
        let mut seen = HashSet::new();
        loop {
            let target = self.resolve(current, index, &mut seen, 0)?;
            if let Some(nested) = current.one("sbaseRef") {
                let submodel = *self.indices[target.1].submodels.get(target.0)?;
                index = self.model_of(submodel)?;
                current = nested;
            } else {
                return Some(target.0);
            }
        }
    }
    fn port(&self, port: View, index: usize) -> Option<&'a str> {
        let mut seen = HashSet::from([port.pk().to_owned()]);
        let target = self.resolve(port, index, &mut seen, 0)?;
        if let Some(nested) = port.one("sbaseRef") {
            self.resolve_into(nested, *self.indices[target.1].submodels.get(target.0)?)
        } else {
            Some(target.0)
        }
    }
}
fn uncert_edges(b: &mut Builder, owner: View, index: &ModelIndex, maths: &Maths) {
    let mut stack = vec![(owner, 0, owner.children("uncertParameters"))];
    while let Some((parent, i, children)) = stack.pop() {
        if let Some(child) = children.get(i).copied() {
            direct_edge(b, parent, child, EdgeKind::UncertParameter);
            reference(b, child, "var", EdgeKind::Var, index);
            reference(b, child, "varLower", EdgeKind::VarLower, index);
            reference(b, child, "varUpper", EdgeKind::VarUpper, index);
            units(b, child, index);
            math(b, child, maths, index);
            stack.push((parent, i + 1, children));
            stack.push((child, 0, child.children("uncertParameters")));
        }
    }
}
pub(super) fn metadata_edges(
    b: &mut Builder,
    universe: &Universe,
    model: &ReportModel,
    i: usize,
    maths: &Maths,
) {
    let index = &universe.indices[i];
    for root in roots(model) {
        for node in with_extensions(root) {
            for (replacement, kind) in node.replacements() {
                direct_edge(b, node, replacement, kind);
                if let Some(submodel) = replacement
                    .string("submodelRef")
                    .and_then(|id| index.base.resolve(id, None))
                    .and_then(|pk| index.submodels.get(pk))
                    .copied()
                {
                    direct_edge(b, replacement, submodel, kind);
                    if let Some(target) = universe.resolve_into(replacement, submodel) {
                        b.edge_pk(replacement.pk(), target, kind);
                    }
                    if let Some(id) = replacement.string("deletion")
                        && let Some(deletion) = submodel
                            .children("listOfDeletions")
                            .into_iter()
                            .find(|d| d.id() == Some(id))
                    {
                        direct_edge(b, replacement, deletion, EdgeKind::Deletion);
                    }
                }
                reference(
                    b,
                    replacement,
                    "conversionFactor",
                    EdgeKind::ConversionFactor,
                    &index.base,
                );
            }
            for uncertainty in node.children("uncertainties") {
                direct_edge(b, node, uncertainty, EdgeKind::Uncertainty);
                uncert_edges(b, uncertainty, &index.base, maths);
            }
            list_edges(b, node);
            if let Some(nested) = node.one("sbaseRef") {
                direct_edge(b, node, nested, EdgeKind::SbaseRef);
            }
        }
    }
}
pub(super) fn reaction_edges(b: &mut Builder, reaction: &ReportReaction, index: &ModelIndex) {
    if let Some(fbc) = &reaction.fbc {
        for (field, kind) in [
            ("lowerFluxBound", EdgeKind::LowerFluxBound),
            ("upperFluxBound", EdgeKind::UpperFluxBound),
        ] {
            if let Some(target) = fbc
                .get(field)
                .and_then(Value::as_str)
                .and_then(|s| index.resolve(s, None))
            {
                b.edge(&reaction.sbase, target, kind);
            }
        }
        if let Some(gpa) = fbc.get("geneProductAssociation").filter(|v| v.is_object()) {
            let mut stack = vec![(reaction.sbase.pk.as_str(), View::Json(gpa))];
            while let Some((parent, node)) = stack.pop() {
                b.edge_pk(parent, node.pk(), EdgeKind::GeneProductAssociation);
                reference(b, node, "geneProduct", EdgeKind::GeneProduct, index);
                let mut children = node.children("associations");
                children.extend(node.one("association"));
                stack.extend(children.into_iter().rev().map(|c| (node.pk(), c)));
            }
        }
    }
}
pub(super) fn model_edges(
    b: &mut Builder,
    universe: &Universe,
    model: &ReportModel,
    i: usize,
    maths: &Maths,
) {
    let index = &universe.indices[i];
    for submodel in model.list_of_submodels.iter().map(View::Json) {
        if let Some(target) = submodel
            .string("modelRef")
            .and_then(|sid| universe.model_refs.get(sid))
        {
            b.edge_pk(submodel.pk(), target, EdgeKind::ModelRef);
        }
        for (field, kind) in [
            ("timeConversionFactor", EdgeKind::TimeConversionFactor),
            ("extentConversionFactor", EdgeKind::ExtentConversionFactor),
        ] {
            reference(b, submodel, field, kind, &index.base);
        }
        for deletion in submodel.children("listOfDeletions") {
            direct_edge(b, submodel, deletion, EdgeKind::Deletion);
            if let Some(target) = universe.resolve_into(deletion, submodel) {
                b.edge_pk(deletion.pk(), target, EdgeKind::Deletion);
            }
        }
    }
    for port in model.list_of_ports.iter().map(View::Json) {
        if let Some(target) = universe.port(port, i) {
            b.edge_pk(port.pk(), target, EdgeKind::Port);
        }
    }
    for gp in model.list_of_gene_products.iter().map(View::Json) {
        reference(
            b,
            gp,
            "associatedSpecies",
            EdgeKind::AssociatedSpecies,
            &index.base,
        );
    }
    if let Some(fbc) = &model.fbc
        && let Some(target) = fbc["activeObjective"]
            .as_str()
            .and_then(|s| index.base.resolve(s, None))
    {
        b.edge(&model.sbase, target, EdgeKind::ActiveObjective);
    }
    for objective in model.list_of_objectives.iter().map(View::Json) {
        for term in objective.children("listOfFluxObjectives") {
            direct_edge(b, objective, term, EdgeKind::FluxObjective);
            reference(b, term, "reaction", EdgeKind::FluxObjective, &index.base);
            reference(b, term, "reaction2", EdgeKind::Reaction2, &index.base);
        }
    }
    for bound in model.list_of_flux_bounds.iter().map(View::Json) {
        reference(b, bound, "reaction", EdgeKind::FluxBound, &index.base);
    }
    for constraint in model
        .list_of_user_defined_constraints
        .iter()
        .map(View::Json)
    {
        reference(
            b,
            constraint,
            "lowerBound",
            EdgeKind::LowerBound,
            &index.base,
        );
        reference(
            b,
            constraint,
            "upperBound",
            EdgeKind::UpperBound,
            &index.base,
        );
        for component in constraint.children("listOfUserDefinedConstraintComponents") {
            direct_edge(b, constraint, component, EdgeKind::ConstraintComponent);
            for (field, kind) in [
                ("variable", EdgeKind::Variable),
                ("variable2", EdgeKind::Variable2),
                ("coefficient", EdgeKind::Coefficient),
            ] {
                reference(b, component, field, kind, &index.base);
            }
        }
    }
    for species in model.list_of_qualitative_species.iter().map(View::Json) {
        reference(
            b,
            species,
            "compartment",
            EdgeKind::Compartment,
            &index.base,
        );
    }
    for transition in model.list_of_transitions.iter().map(View::Json) {
        for (field, kind) in [
            ("listOfInputs", EdgeKind::Input),
            ("listOfOutputs", EdgeKind::Output),
        ] {
            for child in transition.children(field) {
                direct_edge(b, transition, child, kind);
                reference(b, child, "qualitativeSpecies", kind, &index.base);
            }
        }
        for term in transition.children("listOfFunctionTerms") {
            direct_edge(b, transition, term, EdgeKind::FunctionTerm);
            math(b, term, maths, &index.base);
        }
        if let Some(default) = transition.one("defaultTerm") {
            direct_edge(b, transition, default, EdgeKind::DefaultTerm);
        }
    }
}

/// Match package math by its owned XML fragment, which both projections already retain.
/// Unlike a formula string, XML preserves symbol kinds and numeric unit attributes.
pub(super) fn package_maths(report: &Report, document: &Document, maths: &mut Maths) {
    let mut asts = HashMap::new();
    let mut elements: Vec<_> = document.elements().collect();
    let mut extensions = Vec::new();
    loop {
        if let Some(element) = elements.pop() {
            extensions.extend(&element.extensions);
            elements.extend(element.lists.iter().map(|l| &l.element));
        } else if let Some(extension) = extensions.pop() {
            if let (Some(ast), Some(xml)) = (&extension.math, &extension.element.xml) {
                asts.insert(xml.as_str(), ast);
            }
            extensions.extend(&extension.children);
            elements.push(&extension.element);
        } else {
            break;
        }
    }
    for model in &report.models {
        for root in roots(model) {
            for node in with_extensions(root) {
                if let Some(ast) = node.string("xml").and_then(|xml| asts.get(xml)) {
                    let mut owner = SBase::new(node.pk().to_owned(), "SBase");
                    owner.xml = None;
                    add_math(maths, &owner, Some(*ast));
                }
            }
        }
    }
}
