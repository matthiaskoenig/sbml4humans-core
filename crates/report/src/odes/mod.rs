//! Report keys and confined source resolution around shared equation generation.
use crate::{Report, ReportModel};
use sbml4humans_model::{Document, Model};
use sbmlsim_ode::{Links, ModelResolver};

impl<'a> ModelResolver<'a> for crate::source::Context<'a> {
    fn model(&self, entry: usize, model_ref: &str) -> Option<(usize, &'a Model)> {
        self.model(entry, model_ref)
    }
    fn budget(&self) -> usize {
        self.budget
    }
}
fn links(report: &ReportModel) -> Links {
    let mut links = Links::default();
    macro_rules! index {
        ($items:expr) => {
            for object in $items {
                if let Some(sid) = &object.sbase.id {
                    links
                        .symbols
                        .entry(sid.clone())
                        .or_insert_with(|| object.sbase.pk.clone());
                }
            }
        };
    }
    index!(&report.list_of_function_definitions);
    index!(&report.list_of_compartments);
    index!(&report.list_of_species);
    index!(&report.list_of_parameters);
    index!(&report.list_of_reactions);
    index!(&report.list_of_events);
    for r in &report.list_of_reactions {
        index!(&r.list_of_reactants);
        index!(&r.list_of_products);
        index!(&r.list_of_modifiers);
        if let Some(sid) = &r.sbase.id
            && let Some(law) = &r.kinetic_law
        {
            for (i, p) in law.list_of_local_parameters.iter().enumerate() {
                links.locals.insert((sid.clone(), i), p.sbase.pk.clone());
            }
        }
    }
    for base in report
        .list_of_rules
        .iter()
        .map(|r| &r.sbase)
        .chain(report.list_of_reactions.iter().map(|r| &r.sbase))
    {
        if let Some(meta) = &base.meta_id {
            for kind in ["", base.sbml_type] {
                links
                    .metadata
                    .entry((kind.into(), meta.clone()))
                    .or_insert_with(|| base.pk.clone());
            }
        }
    }
    links
}
pub(crate) fn apply(document: &Document, report: &mut Report) {
    apply_in(document, report, None);
}
pub(crate) fn apply_in<'a>(
    document: &'a Document,
    report: &mut Report,
    context: Option<(&crate::source::Context<'a>, usize)>,
) {
    let Some(original) = document.model.as_ref() else {
        return;
    };
    let Some(output) = report.models.first() else {
        return;
    };
    let links = links(output);
    let result = (|| {
        let flattened;
        let model =
            if sbml4humans_model::packages::children(&original.element, "comp", "listOfSubmodels")
                .is_empty()
            {
                original
            } else {
                flattened = sbmlsim_ode::flatten(
                    document,
                    context.map(|(c, i)| (c as &dyn ModelResolver<'a>, i)),
                )?;
                &flattened
            };
        let model = sbmlsim_ode::normalize(document, model);
        sbmlsim_ode::typeset(&model, &links)
    })();
    match result {
        Ok(system) => report.ode_system = Some(system),
        Err(error) => report.ode_error = Some(error),
    }
}
