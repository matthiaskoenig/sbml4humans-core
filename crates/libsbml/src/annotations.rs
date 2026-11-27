//! Copy parsed RDF annotations through borrowed handles while the document is alive.

use sbml4humans_model::{Creator, CvTerm, History};

use crate::{ReadError, strings, sys};

fn malformed(message: &str) -> ReadError {
    ReadError::Annotation(message.to_owned())
}

struct Frame {
    pointer: *mut sys::CVTerm,
    next: u32,
    children: u32,
    term: CvTerm,
}

unsafe fn frame(pointer: *mut sys::CVTerm) -> Result<Frame, ReadError> {
    if pointer.is_null() {
        return Err(malformed("The annotation contains an unreadable CV term."));
    }
    unsafe {
        let qualifier = strings::borrowed(sys::s4h_cvterm_qualifier(pointer)).ok_or_else(|| {
            malformed(&format!(
                "Unsupported qualifier type: '{}'",
                sys::s4h_cvterm_qualifier_type(pointer)
            ))
        })?;
        let resources = (0..sys::s4h_cvterm_num_resources(pointer))
            .map(|n| {
                strings::handed_over(sys::s4h_cvterm_resource(pointer, n))
                    .ok_or_else(|| malformed("An annotation resource could not be read."))
            })
            .collect::<Result<_, _>>()?;
        Ok(Frame {
            pointer,
            next: 0,
            children: sys::s4h_cvterm_num_nested(pointer),
            term: CvTerm {
                qualifier,
                resources,
                nested: vec![],
            },
        })
    }
}

unsafe fn term(pointer: *mut sys::CVTerm) -> Result<CvTerm, ReadError> {
    let mut stack = vec![unsafe { frame(pointer)? }];
    loop {
        let parent = stack
            .last_mut()
            .ok_or_else(|| malformed("The annotation walk lost its root."))?;
        if parent.next < parent.children {
            let child = unsafe { sys::s4h_cvterm_nested(parent.pointer, parent.next) };
            parent.next += 1;
            stack.push(unsafe { frame(child)? });
        } else {
            let completed = stack
                .pop()
                .ok_or_else(|| malformed("The annotation walk lost its root."))?;
            if let Some(parent) = stack.last_mut() {
                parent.term.nested.push(completed.term);
            } else {
                return Ok(completed.term);
            }
        }
    }
}

/// The controlled annotations of an element, preserving term and resource order.
///
/// # Safety
///
/// `sbase` belongs to a live document and the caller holds the libsbml lock.
pub(crate) unsafe fn cvterms(sbase: *mut sys::SBase) -> Result<Vec<CvTerm>, ReadError> {
    unsafe {
        (0..sys::s4h_sbase_num_cvterms(sbase))
            .map(|n| term(sys::s4h_sbase_cvterm(sbase, n)))
            .collect()
    }
}

/// The creators and dates of an element, absent when libsbml reports no history.
///
/// # Safety
///
/// `sbase` belongs to a live document and the caller holds the libsbml lock.
pub(crate) unsafe fn history(sbase: *mut sys::SBase) -> Result<Option<History>, ReadError> {
    unsafe {
        let history = sys::s4h_sbase_history(sbase);
        if history.is_null() {
            return Ok(None);
        }
        let creators = (0..sys::s4h_history_num_creators(history))
            .map(|n| {
                let creator = sys::s4h_history_creator(history, n);
                if creator.is_null() {
                    return Err(malformed("A history creator could not be read."));
                }
                Ok(Creator {
                    given_name: strings::borrowed(sys::s4h_creator_given_name(creator)),
                    family_name: strings::borrowed(sys::s4h_creator_family_name(creator)),
                    organization: strings::borrowed(sys::s4h_creator_organization(creator)),
                    email: strings::borrowed(sys::s4h_creator_email(creator)),
                })
            })
            .collect::<Result<_, _>>()?;
        let modified_dates = (0..sys::s4h_history_num_modified(history))
            .map(|n| {
                strings::borrowed(sys::s4h_history_modified(history, n))
                    .ok_or_else(|| malformed("A history date could not be read."))
            })
            .collect::<Result<_, _>>()?;
        Ok(Some(History {
            creators,
            created_date: strings::borrowed(sys::s4h_history_created(history)),
            modified_dates,
        }))
    }
}
