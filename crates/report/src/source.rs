//! Report construction across the explicitly supplied documents of one source.
use crate::{BuildError, Report};
use sbml4humans_model::{Document, Model};
use serde_json::json;
use std::collections::{HashMap, HashSet};

/// One document and its manifest identity. Resolution never reads additional files.
pub struct SourceDocument<'a> {
    /// The exact location exposed by the source manifest.
    pub location: &'a str,
    /// The already parsed document.
    pub document: &'a Document,
    /// The MD5 of its original bytes, where the source provides one.
    pub checksum: Option<&'a str>,
}

#[derive(Clone, Debug)]
pub(crate) struct Resolution {
    status: &'static str,
    pub(crate) entry: Option<String>,
    pub(crate) target: Option<(usize, usize)>,
    md5_matches: Option<bool>,
}

pub(crate) struct Context<'a> {
    pub(crate) sources: &'a [SourceDocument<'a>],
    pub(crate) resolutions: HashMap<(usize, String), Resolution>,
    pub(crate) budget: usize,
    external_targets: Vec<HashMap<String, Option<(usize, usize)>>>,
    local_models: Vec<HashMap<&'a str, &'a Model>>,
}

/// Normalize an external source path within the archive root.
pub fn normalize_location(path: &str) -> Option<String> {
    let mut parts = Vec::new();
    for part in path.trim_start_matches('/').split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}
fn remote(source: &str) -> bool {
    let Some((scheme, _)) = source.split_once(':') else {
        return false;
    };
    scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}
fn unquote(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(a), Some(b)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            )
        {
            output.push((16 * a + b) as u8);
            i += 3;
        } else {
            output.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&output).into_owned()
}
/// Resolve an external source URI against its referring manifest location.
pub fn resolve_source(location: &str, source: &str) -> Option<String> {
    if source.is_empty() || remote(source) {
        return None;
    }
    let source = unquote(source);
    if source.starts_with('/') {
        return normalize_location(&source);
    }
    let base = normalize_location(location)?;
    let directory = base.rsplit_once('/').map_or("", |(directory, _)| directory);
    normalize_location(&format!("{directory}/{source}"))
}
impl<'a> Context<'a> {
    fn new(
        sources: &'a [SourceDocument<'a>],
        reports: &[Report],
        locations: &[&str],
        budget: usize,
    ) -> Self {
        let mut entries = HashMap::new();
        for location in locations
            .iter()
            .copied()
            .chain(sources.iter().map(|s| s.location))
        {
            if let Some(normalized) = normalize_location(location) {
                entries.insert(normalized, location);
            }
        }
        let by_location: HashMap<_, _> = sources
            .iter()
            .enumerate()
            .map(|(i, s)| (s.location, i))
            .collect();
        let model_ids: Vec<_> = reports
            .iter()
            .map(|report| {
                let mut ids = HashMap::new();
                for (i, model) in report.models.iter().enumerate() {
                    if let Some(id) = model.sbase.id.as_deref() {
                        ids.entry(id).or_insert(i);
                    }
                }
                ids
            })
            .collect();
        let mains: Vec<_> = reports
            .iter()
            .map(|r| r.models.iter().position(|m| m.kind == "model"))
            .collect();
        let external_ids: Vec<_> = reports
            .iter()
            .map(|report| {
                let mut ids = HashMap::new();
                for emd in &report.external_model_definitions {
                    if let Some(id) = emd["id"].as_str() {
                        ids.entry(id).or_insert(emd);
                    }
                }
                ids
            })
            .collect();
        let mut resolutions = HashMap::new();
        for (home, report) in reports.iter().enumerate() {
            for root in &report.external_model_definitions {
                let mut current = (home, root);
                let mut seen = HashSet::new();
                let mut fallback = None;
                let mut checksum = None;
                let mut first = true;
                let result = loop {
                    let (entry, emd) = current;
                    let pk = emd["pk"].as_str().unwrap_or_default();
                    let failure = |status, entry| Resolution {
                        status,
                        entry,
                        target: None,
                        md5_matches: checksum,
                    };
                    if !seen.insert((entry, pk)) {
                        break failure("circular", fallback);
                    }
                    let source = emd["source"].as_str().unwrap_or_default();
                    if remote(source) {
                        break failure("remoteSource", fallback);
                    }
                    let target_location = resolve_source(sources[entry].location, source)
                        .and_then(|location| entries.get(&location).copied());
                    let Some(location) = target_location else {
                        break failure("notFound", fallback);
                    };
                    fallback = Some(location.to_owned());
                    let Some(&target_entry) = by_location.get(location) else {
                        break failure("notSbml", fallback);
                    };
                    if first {
                        checksum = emd["md5"]
                            .as_str()
                            .zip(sources[target_entry].checksum)
                            .map(|(expected, actual)| expected.trim().eq_ignore_ascii_case(actual));
                        first = false;
                    }
                    let model_ref = emd["modelRef"].as_str();
                    let model = model_ref.map_or(mains[target_entry], |id| {
                        model_ids[target_entry].get(id).copied()
                    });
                    if let Some(model) = model {
                        break Resolution {
                            status: "resolved",
                            entry: fallback,
                            target: Some((target_entry, model)),
                            md5_matches: checksum,
                        };
                    }
                    if let Some(next) =
                        model_ref.and_then(|id| external_ids[target_entry].get(id).copied())
                    {
                        current = (target_entry, next);
                    } else {
                        break Resolution {
                            status: "modelNotFound",
                            entry: fallback,
                            target: None,
                            md5_matches: checksum,
                        };
                    }
                };
                resolutions.insert(
                    (home, root["pk"].as_str().unwrap_or_default().to_owned()),
                    result,
                );
            }
        }
        let external_targets = reports
            .iter()
            .enumerate()
            .map(|(entry, report)| {
                report
                    .external_model_definitions
                    .iter()
                    .filter_map(|e| {
                        Some((
                            e["id"].as_str()?.to_owned(),
                            resolutions
                                .get(&(entry, e["pk"].as_str()?.to_owned()))?
                                .target,
                        ))
                    })
                    .collect()
            })
            .collect();
        let local_models = sources
            .iter()
            .map(|source| {
                let mut models = HashMap::new();
                for model in source.document.models() {
                    if let Some(id) = model.element.id.as_deref() {
                        models.entry(id).or_insert(model);
                    }
                }
                models
            })
            .collect();
        Self {
            sources,
            resolutions,
            budget,
            external_targets,
            local_models,
        }
    }
    pub(crate) fn model(&self, entry: usize, model_ref: &str) -> Option<(usize, &'a Model)> {
        if let Some(&model) = self.local_models[entry].get(model_ref) {
            return Some((entry, model));
        }
        let target = self.external_targets[entry].get(model_ref)?.as_ref()?;
        Some((
            target.0,
            self.sources[target.0].document.models().nth(target.1)?,
        ))
    }
}

fn grouped(value: u128) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}
fn expanded_sizes(
    reports: &[Report],
    context: &Context,
    graphs: &[crate::LinkGraph],
) -> Vec<Option<u128>> {
    let mut models = Vec::new();
    let mut identities = HashMap::new();
    let mut mains = Vec::new();
    let mut sizes = Vec::new();
    for (entry, (source, report)) in context.sources.iter().zip(reports).enumerate() {
        let first = models.len();
        mains.push(source.document.model.as_ref().map(|_| first));
        let mut by_scope = HashMap::new();
        for key in graphs[entry].nodes.keys() {
            *by_scope
                .entry(key.split('/').next().unwrap_or_default())
                .or_insert(0u128) += 1;
        }
        for (model, output) in source.document.models().zip(&report.models) {
            identities.insert((entry, std::ptr::from_ref(model) as usize), models.len());
            sizes.push(
                *by_scope
                    .get(output.sbase.pk.split('/').next().unwrap_or_default())
                    .unwrap_or(&0),
            );
            models.push((entry, model));
        }
    }
    let adjacency: Vec<Vec<_>> = models
        .iter()
        .map(|(entry, model)| {
            crate::packages::children(&model.element, "comp", "listOfSubmodels")
                .into_iter()
                .filter_map(|submodel| {
                    context.model(*entry, crate::packages::attr(submodel, "modelRef")?)
                })
                .filter_map(|(entry, model)| {
                    identities
                        .get(&(entry, std::ptr::from_ref(model) as usize))
                        .copied()
                })
                .collect()
        })
        .collect();
    mains
        .into_iter()
        .enumerate()
        .map(|(entry, main)| {
            let main = main?;
            // Explicit postorder traversal bounds stack use independently of model nesting.
            let mut stack = vec![(main, 0, 0u128)];
            let mut chain = HashSet::from([main]);
            let mut cached = HashMap::new();
            let total = loop {
                let (current, next, total) = stack.last_mut()?;
                if let Some(&child) = adjacency[*current].get(*next) {
                    *next += 1;
                    if chain.contains(&child) {
                        continue;
                    }
                    if let Some(subtotal) = cached.get(&child) {
                        *total = total.saturating_add(sizes[child]).saturating_add(*subtotal);
                    } else {
                        chain.insert(child);
                        stack.push((child, 0, 0));
                    }
                } else {
                    let (current, _, subtotal) = stack.pop()?;
                    chain.remove(&current);
                    cached.insert(current, subtotal);
                    if let Some((_, _, total)) = stack.last_mut() {
                        *total = total
                            .saturating_add(sizes[current])
                            .saturating_add(subtotal);
                    } else {
                        break subtotal;
                    }
                }
            };
            (total > 0).then(|| total.saturating_add(graphs[entry].nodes.len() as u128))
        })
        .collect()
}

/// Build complete reports together, resolving only the documents explicitly supplied.
///
/// # Errors
/// Returns the same report construction errors as the single-document builder.
pub fn build_many(
    sources: &[SourceDocument<'_>],
    locations: &[&str],
    budget: usize,
) -> Result<Vec<Report>, BuildError> {
    let mut reports = sources
        .iter()
        .map(|s| crate::build::unlinked(s.document))
        .collect::<Result<Vec<_>, _>>()?;
    let context = Context::new(sources, &reports, locations, budget);
    let model_keys: Vec<Vec<_>> = reports
        .iter()
        .map(|r| r.models.iter().map(|m| m.sbase.pk.clone()).collect())
        .collect();
    for (entry, report) in reports.iter_mut().enumerate() {
        for emd in &mut report.external_model_definitions {
            let resolution =
                &context.resolutions[&(entry, emd["pk"].as_str().unwrap_or_default().to_owned())];
            let model = resolution
                .target
                .map(|(entry, model)| model_keys[entry][model].as_str());
            emd["resolution"] = json!({"status":resolution.status,"entry":resolution.entry,"model":model,"md5Matches":resolution.md5_matches});
        }
    }
    let graphs = crate::links::build_many(&reports, sources);
    let expanded = expanded_sizes(&reports, &context, &graphs);
    for ((entry, report), graph) in reports.iter_mut().enumerate().zip(graphs) {
        report.link_graph = graph;
        if let Some(size) = expanded[entry].filter(|size| *size > budget as u128) {
            report.ode_error = Some(format!(
                "The equations of the model were not built: it expands by its comp submodels to {} elements, more than the {} elements a model may expand to.",
                grouped(size),
                grouped(budget as u128)
            ));
        } else {
            crate::odes::apply_in(sources[entry].document, report, Some((&context, entry)));
        }
    }
    Ok(reports)
}
