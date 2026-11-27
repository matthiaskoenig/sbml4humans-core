//! Native source summaries and compressed report transport.
use crate::{ReportError, SourceOptions, SourceResponse};
use serde::Serialize;
use std::{io::Write, path::Path};

/// Serialized report data and small diagnostics, without host-side JSON traversal.
pub struct ReportTransport {
    /// Report JSON, optionally gzip compressed.
    pub content: Vec<u8>,
    /// Equation diagnostics in source order.
    pub diagnostics: Vec<String>,
}
/// Serialize a response once, optionally directly into gzip.
///
/// # Errors
/// Returns serialization or compression failures.
pub fn encode_report(
    response: &SourceResponse,
    gzip: bool,
) -> Result<ReportTransport, ReportError> {
    let diagnostics = response
        .reports
        .values()
        .filter_map(|entry| entry.report.ode_error.as_deref())
        .map(|message| message.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    let content = if gzip {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(5));
        serde_json::to_writer(&mut encoder, response).map_err(internal)?;
        encoder.flush().map_err(internal)?;
        encoder.finish().map_err(internal)?
    } else {
        serde_json::to_vec(response).map_err(internal)?
    };
    Ok(ReportTransport {
        content,
        diagnostics,
    })
}
fn internal(error: impl std::fmt::Display) -> ReportError {
    ReportError {
        kind: "internal",
        message: error.to_string(),
    }
}
fn malformed(error: impl std::fmt::Display) -> ReportError {
    ReportError {
        kind: "malformed",
        message: error.to_string(),
    }
}
/// Metadata consumed by the example API, without constructing reports or equations.
#[derive(Serialize)]
pub struct ExampleSummary {
    /// Public example identifier.
    pub id: String,
    /// The selected entry of a biomodel archive.
    pub location: Option<String>,
    /// Model name or archive filename stem.
    pub name: Option<String>,
    /// Model notes or archive contents sentence.
    pub description: Option<String>,
    /// Declared SBML package prefixes, or OMEX.
    pub packages: Vec<String>,
}
/// Read metadata for a standalone model, whole archive, or its main biomodel.
///
/// # Errors
/// Refuses malformed archives, unreadable models and unknown summary modes.
pub fn example_summary(
    path: &Path,
    mode: &str,
    example_id: Option<&str>,
) -> Result<ExampleSummary, ReportError> {
    let bytes = std::fs::read(path).map_err(malformed)?;
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| malformed("The example filename is not UTF-8."))?;
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("model");
    let limits = crate::SourceLimits {
        max_content_size: usize::MAX,
        max_archive_entries: usize::MAX,
        max_compression_ratio: u64::MAX,
        ..Default::default()
    };
    if mode == "archive" || mode == "biomodel" {
        let (reader, entries, members) = crate::source::archive_members(&bytes, &limits)?
            .ok_or_else(|| malformed("The source is not a COMBINE archive."))?;
        let entries: Vec<_> = entries
            .iter()
            .filter(|entry| crate::source::sbml(&entry.format))
            .collect();
        if mode == "archive" {
            let names = entries
                .iter()
                .map(|entry| {
                    Path::new(&entry.location)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                })
                .collect::<Vec<_>>()
                .join(", ");
            let mut description = if entries.is_empty() {
                "COMBINE archive without an SBML entry".to_owned()
            } else {
                format!(
                    "COMBINE archive with {} SBML {}",
                    entries.len(),
                    if entries.len() == 1 {
                        "entry"
                    } else {
                        "entries"
                    }
                )
            };
            if !entries.is_empty() && names.chars().count() <= 60 {
                description.push_str(": ");
                description.push_str(&names);
            }
            return Ok(ExampleSummary {
                id: example_id.unwrap_or(stem).into(),
                location: None,
                name: Some(stem.into()),
                description: Some(description),
                packages: vec!["OMEX".into()],
            });
        }
        let entry = entries
            .iter()
            .find(|entry| entry.master)
            .copied()
            .or_else(|| entries.iter().min_by_key(|entry| &entry.location).copied())
            .ok_or_else(|| malformed("The archive contains no SBML entry"))?;
        let bytes = reader
            .inflate(&reader.members()[members[&entry.location]])
            .map_err(malformed)?;
        let mut summary = model_summary(&bytes, filename, stem, example_id)?;
        summary.location = Some(entry.location.clone());
        return Ok(summary);
    }
    if mode != "sbml" {
        return Err(malformed("Unknown example metadata mode."));
    }
    model_summary(&bytes, filename, stem, example_id)
}
fn model_summary(
    bytes: &[u8],
    filename: &str,
    stem: &str,
    example_id: Option<&str>,
) -> Result<ExampleSummary, ReportError> {
    let bytes = crate::decode(bytes, usize::MAX)?;
    let document = sbml4humans_reader::read(&bytes).map_err(malformed)?;
    let model = document
        .model
        .as_ref()
        .ok_or_else(|| malformed("Model could not be read"))?;
    let id = model
        .element
        .id
        .as_deref()
        .filter(|id| !id.is_empty())
        .unwrap_or(stem);
    Ok(ExampleSummary {
        id: example_id
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{id} ({filename})")),
        location: None,
        name: model.element.name.clone(),
        description: model.element.notes.clone(),
        packages: document
            .packages
            .iter()
            .map(|package| package.prefix.clone())
            .collect(),
    })
}
/// Select an example's archived model with companions confined to its own directory.
///
/// # Errors
/// Refuses invalid selections, malformed archives and unreadable selected models.
pub fn prepare_host_path(
    path: &Path,
    trusted: bool,
    options: &SourceOptions,
    entry: Option<&str>,
) -> Result<crate::PreparedSource, ReportError> {
    let Some(entry) = entry else {
        return crate::prepare_path(path, trusted, options);
    };
    if !trusted {
        return Err(malformed(
            "Selecting an example entry requires a trusted source.",
        ));
    }
    let bytes = std::fs::read(path).map_err(malformed)?;
    let mut options = options.clone();
    options.limits = crate::SourceLimits {
        max_content_size: usize::MAX,
        max_archive_entries: usize::MAX,
        max_compression_ratio: u64::MAX,
        ..Default::default()
    };
    let (reader, _, members) = crate::source::archive_members(&bytes, &options.limits)?
        .ok_or_else(|| malformed("The source is not a COMBINE archive."))?;
    let selected = pymetadata_core::location::normalize_location(entry).map_err(malformed)?;
    let index = members
        .get(&selected)
        .ok_or_else(|| malformed(format!("The source has no SBML entry '{entry}'.")))?;
    let bytes = reader
        .inflate(&reader.members()[*index])
        .map_err(malformed)?;
    let source_path = Path::new(&selected);
    let root = source_path.parent().unwrap_or(Path::new(""));
    let filename = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| malformed("The selected filename is not UTF-8."))?;
    options.location = format!(
        "./{}",
        if bytes.starts_with(&[31, 139]) {
            filename.strip_suffix(".gz").unwrap_or(filename)
        } else {
            filename
        }
    );
    let mut prepared = crate::prepare(&bytes, &options)?;
    prepared.admit_companions(&options, |location| {
        let global =
            pymetadata_core::location::normalize_location(&root.join(location).to_string_lossy())
                .map_err(malformed)?;
        members
            .get(&global)
            .map(|index| reader.inflate(&reader.members()[*index]).map_err(malformed))
            .transpose()
    })?;
    Ok(prepared)
}

/// Build a native report and encode it while its typed data is available.
///
/// # Errors
/// Returns input, report or serialization failures.
pub fn report_path(
    path: &Path,
    trusted: bool,
    options: &SourceOptions,
    gzip: bool,
    entry: Option<&str>,
) -> Result<ReportTransport, ReportError> {
    let start = std::time::Instant::now();
    let prepared = prepare_host_path(path, trusted, options, entry)?;
    let mut response = prepared.report(options)?;
    let elapsed = format!("{:.3} [s]", start.elapsed().as_secs_f64());
    for entry in response.reports.values_mut() {
        entry.debug.json_report_time = elapsed.clone();
    }
    encode_report(&response, gzip)
}
