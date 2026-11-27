//! Owned native validation sessions with a closed set of companion documents.
use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{fill::locked, strings, sys};
use sbml4humans_model::locations::{normalize_location, resolve_source};

/// Source positions supplied by the report's canonical element-key builder.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementPositions {
    /// Primary key used when no element precedes an issue.
    pub document_pk: String,
    /// `(line, column, depth, primary key)`, ordered as in the original document.
    pub elements: Vec<(u32, u32, u32, String)>,
}
impl ElementPositions {
    fn complete(&mut self, document: *mut sys::SBMLDocument) -> Result<(), ValidationError> {
        type Position = (u32, u32, u32, u32, u32, bool);
        extern "C" fn visit(
            context: *mut c_void,
            line: u32,
            column: u32,
            depth: u32,
            parent_line: u32,
            parent_column: u32,
            list: c_int,
        ) {
            // SAFETY: The synchronous bridge receives this live Vec and keeps no pointer.
            let positions = unsafe { &mut *context.cast::<Vec<Position>>() };
            positions.push((line, column, depth, parent_line, parent_column, list != 0));
        }
        let keys: HashMap<_, _> = self
            .elements
            .iter()
            .map(|p| ((p.0, p.1), p.3.clone()))
            .collect();
        let mut positions: Vec<Position> = Vec::new();
        // SAFETY: The session owns this document and holds the global library lock.
        status(unsafe {
            s4hv_positions(
                document,
                (&mut positions as *mut Vec<Position>).cast(),
                visit,
            )
        })?;
        positions.sort_by_key(|p| p.2);
        self.elements.clear();
        for (line, column, depth, parent_line, parent_column, list) in positions {
            if line == 0 {
                continue;
            }
            let key = keys.get(&(line, column)).or_else(|| {
                list.then(|| keys.get(&(parent_line, parent_column)))
                    .flatten()
            });
            if let Some(key) = key {
                self.elements.push((line, column, depth, key.clone()));
            }
        }
        Ok(())
    }
    fn prepare(&mut self) {
        self.elements.sort_by_key(|a| (a.0, a.1, a.2));
    }
    fn at(&self, line: u32, column: u32) -> String {
        if line == 0 {
            return self.document_pk.clone();
        }
        let index = self
            .elements
            .partition_point(|p| (p.0, p.1) <= (line, column));
        index
            .checked_sub(1)
            .map_or_else(|| self.document_pk.clone(), |i| self.elements[i].3.clone())
    }
}

/// One admitted SBML source; this interface never discovers files or URLs.
pub struct ValidationSource {
    /// Location relative to the source manifest.
    pub location: String,
    /// Original admitted content, preserving authored numbers and source coordinates.
    pub content: ValidationContent,
    /// Original location URI, needed for libSBML's comp resolver.
    pub uri: String,
    /// Element keys and source positions from the report.
    pub positions: ElementPositions,
}

/// Explicitly admitted source content; no dependency is discovered from a path.
pub enum ValidationContent {
    /// XML bytes supplied directly by a native caller.
    Xml(Vec<u8>),
    /// A file admitted by the host, kept alive until the session has parsed it.
    Path(PathBuf),
}

/// A diagnostic in the application's existing validation contract.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    /// libSBML rule identifier.
    pub rule: u32,
    /// `error`, `warning` or `info`.
    pub severity: &'static str,
    /// libSBML diagnostic category.
    pub category: String,
    /// Short diagnostic description.
    pub short_message: String,
    /// Complete diagnostic, with surrounding whitespace removed.
    pub message: String,
    /// Source line.
    pub line: u32,
    /// Source column.
    pub column: u32,
    /// Key of the element which precedes the issue.
    pub pk: String,
}

/// A native validation failure, distinct from ordinary model diagnostics.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    /// An input or bridge failure.
    #[error("{0}")]
    Invalid(String),
    /// libSBML failed to allocate memory.
    #[error("libSBML ran out of memory during validation.")]
    Memory,
}
fn status(value: c_int) -> Result<(), ValidationError> {
    match value {
        0 => Ok(()),
        -2 => Err(ValidationError::Memory),
        _ => Err(ValidationError::Invalid(
            "libSBML could not complete validation.".into(),
        )),
    }
}

#[repr(C)]
#[derive(Default)]
struct Issue {
    rule: c_uint,
    severity: c_uint,
    line: c_uint,
    column: c_uint,
    category: *mut c_char,
    short_message: *mut c_char,
    message: *mut c_char,
}
type Lookup = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    *const c_char,
    c_int,
) -> *mut sys::SBMLDocument;
unsafe extern "C" {
    fn s4hv_read_file(path: *const c_char, status: *mut c_int) -> *mut sys::SBMLDocument;
    fn s4hv_read_xml(xml: *const c_char, status: *mut c_int) -> *mut sys::SBMLDocument;
    fn s4hv_location(doc: *mut sys::SBMLDocument, uri: *const c_char) -> c_int;
    fn s4hv_check(doc: *mut sys::SBMLDocument, context: *mut c_void, lookup: Lookup) -> c_int;
    fn s4hv_positions(
        doc: *mut sys::SBMLDocument,
        context: *mut c_void,
        visit: extern "C" fn(*mut c_void, u32, u32, u32, u32, u32, c_int),
    ) -> c_int;
    fn s4hv_rule(code: u32, package_only: c_int, issue: *mut Issue) -> c_int;
    fn s4hv_count(doc: *mut sys::SBMLDocument) -> c_uint;
    fn s4hv_issue(doc: *mut sys::SBMLDocument, index: c_uint, issue: *mut Issue) -> c_int;
}

struct Document {
    pointer: *mut sys::SBMLDocument,
    location: String,
    uri: String,
    positions: ElementPositions,
    checked: bool,
}
impl Drop for Document {
    fn drop(&mut self) {
        unsafe { sys::s4h_document_free(self.pointer) };
    }
}

/// Documents parsed once and reused while archive entries are checked individually.
///
/// Every libSBML access, including destruction and resolver installation, holds
/// the process-wide lock. Hosts must additionally retain process isolation to
/// bound native crashes, validation time and memory use.
pub struct ValidationSession {
    documents: Vec<Document>,
    locations: HashMap<String, usize>,
    uris: HashMap<String, usize>,
}
// SAFETY: The session owns its documents, exposes no pointers, and serializes all
// native access through the global libSBML lock. It may move between host threads.
unsafe impl Send for ValidationSession {}

impl Drop for ValidationSession {
    fn drop(&mut self) {
        let _lock = locked();
        self.documents.clear();
    }
}
impl ValidationSession {
    /// Parse an explicit, closed source set without performing consistency checks.
    ///
    /// # Errors
    /// Refuses invalid locations, duplicate normalized names, NUL bytes, sources
    /// without models, and native allocation or parsing failures.
    pub fn new(sources: Vec<ValidationSource>) -> Result<Self, ValidationError> {
        Self::create(sources, false)
    }

    /// Complete Rust-authored report positions with native list owners and depths.
    ///
    /// # Errors
    /// Returns the same input and native failures as [`Self::new`].
    pub fn from_report(sources: Vec<ValidationSource>) -> Result<Self, ValidationError> {
        Self::create(sources, true)
    }

    fn create(
        sources: Vec<ValidationSource>,
        complete_positions: bool,
    ) -> Result<Self, ValidationError> {
        let mut session = Self {
            documents: Vec::new(),
            locations: HashMap::new(),
            uris: HashMap::new(),
        };
        // Keep the lock in a nested scope so error cleanup can acquire it again.
        let result = (|| {
            let _lock = locked();
            let mut duplicates = HashSet::new();
            for source in sources {
                let location = normalize_location(&source.location).ok_or_else(|| {
                    ValidationError::Invalid("Invalid validation source location.".into())
                })?;
                if session.locations.contains_key(&location) {
                    return Err(ValidationError::Invalid(
                        "Duplicate validation source location.".into(),
                    ));
                }
                let uri = CString::new(source.uri.as_str()).map_err(|_| {
                    ValidationError::Invalid("Source URI contains a NUL byte.".into())
                })?;
                let mut read_status = 0;
                let pointer = match source.content {
                    ValidationContent::Xml(xml) => {
                        let xml = CString::new(xml).map_err(|_| {
                            ValidationError::Invalid("SBML XML string contains a NUL byte.".into())
                        })?;
                        unsafe { s4hv_read_xml(xml.as_ptr(), &mut read_status) }
                    }
                    ValidationContent::Path(path) => {
                        let path = path.to_str().ok_or_else(|| {
                            ValidationError::Invalid("Validation path is not UTF-8.".into())
                        })?;
                        let path = CString::new(path).map_err(|_| {
                            ValidationError::Invalid("Validation path contains a NUL byte.".into())
                        })?;
                        unsafe { s4hv_read_file(path.as_ptr(), &mut read_status) }
                    }
                };
                status(read_status)?;
                if pointer.is_null() {
                    return Err(ValidationError::Invalid("libSBML read no document.".into()));
                }
                let mut document = Document {
                    pointer,
                    location: source.location,
                    uri: source.uri,
                    positions: source.positions,
                    checked: false,
                };
                if unsafe { sys::s4h_document_out_of_memory(pointer) } != 0 {
                    return Err(ValidationError::Memory);
                }
                if unsafe { sys::s4h_document_model(pointer) }.is_null() {
                    let log = unsafe { strings::handed_over(sys::s4h_document_error_log(pointer)) }
                        .unwrap_or_default();
                    return Err(ValidationError::Invalid(format!(
                        "No SBML model could be read:\n{log}"
                    )));
                }
                status(unsafe { s4hv_location(pointer, uri.as_ptr()) })?;
                if complete_positions {
                    document.positions.complete(pointer)?;
                }
                document.positions.prepare();
                let index = session.documents.len();
                session.locations.insert(location, index);
                if !document.uri.is_empty()
                    && session.uris.insert(document.uri.clone(), index).is_some()
                {
                    duplicates.insert(document.uri.clone());
                }
                session.documents.push(document);
            }
            for uri in duplicates {
                session.uris.remove(&uri);
            }
            Ok(())
        })();
        result?;
        Ok(session)
    }

    /// Read parser diagnostics, optionally running the consistency checks once.
    ///
    /// # Errors
    /// Returns native failures or an unknown source location. Model errors are
    /// successful results and retain libSBML's order and diagnostic text.
    pub fn issues(
        &mut self,
        location: &str,
        check: bool,
    ) -> Result<Vec<ValidationIssue>, ValidationError> {
        let index = normalize_location(location)
            .and_then(|s| self.locations.get(&s).copied())
            .ok_or_else(|| {
                ValidationError::Invalid("Unknown validation source location.".into())
            })?;
        let _lock = locked();
        let pointer = self.documents[index].pointer;
        if check && !self.documents[index].checked {
            let mut context = Resolver {
                session: self,
                current: index,
            };
            status(unsafe {
                s4hv_check(pointer, (&mut context as *mut Resolver<'_>).cast(), lookup)
            })?;
            self.documents[index].checked = true;
        }
        let positions = &self.documents[index].positions;
        let mut seen = HashSet::new();
        let mut issues = Vec::new();
        for index in 0..unsafe { s4hv_count(pointer) } {
            let mut raw = Issue::default();
            status(unsafe { s4hv_issue(pointer, index, &mut raw) })?;
            let category = unsafe { strings::handed_over(raw.category) }.unwrap_or_default();
            let short_message =
                unsafe { strings::handed_over(raw.short_message) }.unwrap_or_default();
            let message = unsafe { strings::handed_over(raw.message) }
                .unwrap_or_default()
                .trim()
                .to_owned();
            if !seen.insert((raw.rule, raw.line, raw.column, message.clone())) {
                continue;
            }
            issues.push(ValidationIssue {
                rule: raw.rule,
                severity: match raw.severity {
                    2 => "error",
                    1 => "warning",
                    _ => "info",
                },
                category,
                short_message,
                message,
                line: raw.line,
                column: raw.column,
                pk: positions.at(raw.line, raw.column),
            });
        }
        Ok(issues)
    }
}

struct Resolver<'a> {
    session: &'a ValidationSession,
    current: usize,
}
unsafe extern "C" fn lookup(
    context: *mut c_void,
    uri: *const c_char,
    base: *const c_char,
    unique_uri: c_int,
) -> *mut sys::SBMLDocument {
    // No callback may unwind into C++. All inputs are borrowed from this check.
    let context = unsafe { &*context.cast::<Resolver<'_>>() };
    let uri = unsafe { CStr::from_ptr(uri) }.to_string_lossy();
    let base = unsafe { CStr::from_ptr(base) }.to_string_lossy();
    let session = context.session;
    let home = &session.documents[context.current];
    let base = if base == home.uri {
        Some(home.location.as_str())
    } else {
        session
            .uris
            .get(base.as_ref())
            .map(|i| session.documents[*i].location.as_str())
    };
    let target = base
        .and_then(|b| resolve_source(b, &uri))
        .and_then(|p| session.locations.get(&p).copied());
    target
        .filter(|i| unique_uri == 0 || session.uris.get(&session.documents[*i].uri) == Some(i))
        .map_or(std::ptr::null_mut(), |i| session.documents[i].pointer)
}

/// One validation rule, read from the same native tables as consistency checks.
#[derive(Debug, Serialize)]
pub struct ValidationRule {
    /// The libSBML error identifier.
    pub id: u32,
    /// The core or package specification.
    pub package: String,
    /// Error or warning.
    pub severity: &'static str,
    /// Whitespace-normalized rule text.
    pub message: String,
    /// The trailing specification reference, when stated.
    pub section: Option<String>,
}
fn rule_data(code: u32, package_only: bool) -> Result<(String, String, u32), ValidationError> {
    let _lock = locked();
    let mut issue = Issue::default();
    // SAFETY: The bridge fills owned strings and catches native exceptions.
    let result = unsafe { s4hv_rule(code, c_int::from(package_only), &mut issue) };
    if result == 1 {
        return Err(ValidationError::Invalid(format!(
            "{code} is not a validation rule of libsbml"
        )));
    }
    status(result)?;
    // SAFETY: Both strings were allocated by the bridge with its shared allocator.
    let package = unsafe { strings::handed_over(issue.category) }.unwrap_or_default();
    let message = unsafe { strings::handed_over(issue.message) }.unwrap_or_default();
    Ok((package, message, issue.severity))
}
/// Identify the specification a rule number belongs to, without resolving its text.
///
/// # Errors
/// Refuses numbers below the validation-rule range and native failures.
pub fn rule_package(code: u32) -> Result<String, ValidationError> {
    rule_data(code, true).map(|data| data.0)
}
/// Resolve the newest applicable rule from native core and package tables.
///
/// # Errors
/// Refuses unknown rule identifiers and native failures.
pub fn validation_rule(code: u32) -> Result<ValidationRule, ValidationError> {
    let (package, message, severity) = rule_data(code, false)?;
    let message = message.split_whitespace().collect::<Vec<_>>().join(" ");
    let (message, section) = if let Some((message, reference)) = message.split_once("Reference:") {
        (
            message.trim_end().to_owned(),
            Some(reference.trim().to_owned()),
        )
    } else {
        (message, None)
    };
    Ok(ValidationRule {
        id: code,
        package,
        severity: if severity == 1 { "warning" } else { "error" },
        message,
        section,
    })
}
