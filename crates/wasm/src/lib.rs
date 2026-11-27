//! Import-free WebAssembly interface. Each worker owns one instance and its buffers.

use sbml4humans_core::{MAX_INPUT_BYTES, ReportError};
use std::cell::RefCell;

#[derive(Default)]
struct Buffers {
    input: Vec<u8>,
    output: Vec<u8>,
}
thread_local! {static BUFFERS:RefCell<Buffers>=RefCell::new(Buffers::default());}

fn fail(buffers: &mut Buffers, error: ReportError) -> i32 {
    buffers.output = serde_json::to_vec(&error).unwrap_or_else(|_| {
        b"{\"kind\":\"internal\",\"message\":\"The error could not be encoded.\"}".to_vec()
    });
    1
}

/// The version of this module's buffer and status-code interface.
#[unsafe(no_mangle)]
pub extern "C" fn report_abi_version() -> u32 {
    1
}

/// Allocate input owned by the instance; zero means refused or unavailable memory.
#[unsafe(no_mangle)]
pub extern "C" fn report_alloc(length: usize) -> *mut u8 {
    BUFFERS.with(|state| {
        let mut state = state.borrow_mut();
        state.input.clear();
        state.output.clear();
        if length > MAX_INPUT_BYTES {
            fail(
                &mut state,
                ReportError::refused("The SBML source exceeds the input budget."),
            );
            return std::ptr::null_mut();
        }
        if state.input.try_reserve(length).is_err() {
            fail(
                &mut state,
                ReportError::refused("Memory for the SBML source could not be allocated."),
            );
            return std::ptr::null_mut();
        }
        state.input.resize(length, 0);
        if length == 0 {
            std::ptr::null_mut()
        } else {
            state.input.as_mut_ptr()
        }
    })
}

/// Parse the current input and publish JSON. Returns zero on success, one on error.
#[unsafe(no_mangle)]
pub extern "C" fn report_build() -> i32 {
    BUFFERS.with(|state| {
        let mut state = state.borrow_mut();
        let result = sbml4humans_core::report_json(&state.input);
        state.input.clear();
        match result {
            Ok(output) => {
                state.output = output;
                0
            }
            Err(error) => fail(&mut state, error),
        }
    })
}

/// Start address of the current output, valid until the next mutating call.
#[unsafe(no_mangle)]
pub extern "C" fn report_output_ptr() -> *const u8 {
    BUFFERS.with(|state| {
        let state = state.borrow();
        if state.output.is_empty() {
            std::ptr::null()
        } else {
            state.output.as_ptr()
        }
    })
}
/// Length of the current output in bytes.
#[unsafe(no_mangle)]
pub extern "C" fn report_output_len() -> usize {
    BUFFERS.with(|state| state.borrow().output.len())
}
/// Release input and output allocations without retaining a document between calls.
#[unsafe(no_mangle)]
pub extern "C" fn report_release() {
    BUFFERS.with(|state| *state.borrow_mut() = Buffers::default());
}

#[cfg(test)]
mod tests {
    use super::*;
    fn call(bytes: &[u8]) -> (i32, serde_json::Value) {
        let ptr = report_alloc(bytes.len());
        assert!(!ptr.is_null());
        // The input buffer belongs to this thread's instance and has exactly this length.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        }
        let status = report_build();
        let output =
            unsafe { std::slice::from_raw_parts(report_output_ptr(), report_output_len()) };
        (status, serde_json::from_slice(output).unwrap())
    }
    #[test]
    fn invalid_then_valid_input_does_not_reuse_stale_state() {
        let (status, error) = call(b"bad XML");
        assert_eq!(status, 1);
        assert_eq!(error["kind"], "malformed");
        let (status, report) = call(include_bytes!("../../../testdata/level2_graph_units.xml"));
        assert_eq!(status, 0);
        assert_eq!(report["models"][0]["id"], "l2_units");
        report_release();
        assert_eq!(report_output_len(), 0);
    }
    #[test]
    fn a_refused_allocation_drops_previous_input() {
        assert!(report_alloc(MAX_INPUT_BYTES + 1).is_null());
        assert_eq!(report_build(), 1);
    }
}
