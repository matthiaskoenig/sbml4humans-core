//! The strings of libsbml as owned strings of Rust.
//!
//! Neither conversion can fail, so that no string of a document can end a
//! read: a null pointer, which is how the shim answers an attribute a document
//! does not set, and an empty string are both nothing, and bytes which are no
//! UTF-8 are converted lossily rather than dropped.

use std::ffi::{CStr, c_char};

use crate::sys;

/// The text of a string libsbml lends, `None` when there is none.
///
/// # Safety
///
/// `text` is null or a nul terminated string of a document which is alive.
pub(crate) unsafe fn borrowed(text: *const c_char) -> Option<String> {
    if text.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(text) }.to_bytes();
    if bytes.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(bytes).into_owned())
    }
}

/// The text of a string libsbml handed over, which this frees.
///
/// # Safety
///
/// `text` is null or a nul terminated string of a function of the shim which
/// hands its string over, and nothing else frees it.
pub(crate) unsafe fn handed_over(text: *mut c_char) -> Option<String> {
    let owned = unsafe { borrowed(text) };
    if !text.is_null() {
        unsafe { sys::s4h_string_free(text) };
    }
    owned
}
