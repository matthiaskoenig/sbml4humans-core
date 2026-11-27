//! Operator-selected paths use the same source reader with bounded local companions.
use crate::{PreparedSource, ReportError, SourceOptions, prepare};
use std::{io::Read, path::Path};

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
    prepared.admit_companions(&options, |location| {
        let Ok(path) = root.join(location).canonicalize() else {
            return Ok(None);
        };
        if !path.starts_with(&root) || !path.is_file() {
            return Ok(None);
        }
        std::fs::read(path).map(Some).map_err(input_error)
    })?;
    Ok(prepared)
}
