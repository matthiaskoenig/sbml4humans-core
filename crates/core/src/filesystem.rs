//! Operator-selected paths use the same source reader with bounded local companions.
use crate::{PreparedSource, ReportError, SourceOptions, prepare};
use pymetadata_core::{formats::lookup_format, manifest::Entry};
use sbml4humans_model::{
    locations::resolve_source,
    packages::{attr, children},
};
use std::{collections::HashSet, io::Read, path::Path};

fn input_error(error: impl std::fmt::Display) -> ReportError {
    ReportError {
        kind: "malformed",
        message: error.to_string(),
    }
}

/// Admit a file and, only for a trusted standalone file, its local comp companions.
/// Symlinks and normalized references must remain inside the selected directory.
/// Archives resolve exclusively within their own members, even when trusted.
///
/// # Errors
/// Returns file, input-limit, archive or SBML reader failures.
pub fn prepare_path(
    path: &Path,
    trusted: bool,
    options: &SourceOptions,
) -> Result<PreparedSource, ReportError> {
    let mut options = options.clone();
    if trusted {
        options.limits.max_content_size = usize::MAX;
        options.limits.max_archive_entries = usize::MAX;
        options.limits.max_compression_ratio = u64::MAX;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| input_error("The source filename is not UTF-8."))?;
        options.location = format!("./{name}");
    }
    let mut source = Vec::new();
    std::fs::File::open(path)
        .map_err(input_error)?
        .take((options.limits.max_content_size as u64).saturating_add(1))
        .read_to_end(&mut source)
        .map_err(input_error)?;
    if trusted && source.starts_with(&[0x1f, 0x8b]) {
        options.location = options
            .location
            .strip_suffix(".gz")
            .unwrap_or(&options.location)
            .to_owned();
    }
    // `prepare` alone owns ZIP recognition, manifest handling and decompression limits.
    let mut prepared = prepare(&source, &options)?;
    let standalone = prepared.entries.len() == 1
        && prepared.entries[0].location == options.location
        && !prepared.archive;
    if !trusted || !standalone {
        return Ok(prepared);
    }
    let root = path
        .parent()
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(input_error)?;
    let mut seen: HashSet<_> = prepared
        .entries
        .iter()
        .filter_map(|entry| sbml4humans_model::locations::normalize_location(&entry.location))
        .collect();
    let mut next = 0;
    while next < prepared.entries.len() {
        let entry = &prepared.entries[next];
        let companions: Vec<_> = children(
            &entry.document.element,
            "comp",
            "listOfExternalModelDefinitions",
        )
        .into_iter()
        .filter_map(|definition| resolve_source(&entry.location, attr(definition, "source")?))
        .collect();
        next += 1;
        for location in companions {
            if !seen.insert(location.clone()) {
                continue;
            }
            let Ok(path) = root.join(&location).canonicalize() else {
                continue;
            };
            if !path.starts_with(&root) || !path.is_file() {
                continue;
            }
            let location = format!("./{location}");
            let mut companion_options = options.clone();
            companion_options.location = location.clone();
            let companion = std::fs::read(&path)
                .map_err(input_error)
                .and_then(|bytes| prepare(&bytes, &companion_options));
            match companion {
                Ok(mut companion) if !companion.archive && companion.entries.len() == 1 => {
                    prepared.manifest.push(
                        Entry::new(&location, lookup_format("sbml"), false).map_err(input_error)?,
                    );
                    prepared.entries.append(&mut companion.entries);
                }
                _ => prepared.unavailable.push(location),
            }
        }
    }
    Ok(prepared)
}
