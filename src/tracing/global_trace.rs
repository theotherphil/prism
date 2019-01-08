//! An assortment of awful hacks to allow tracing of generated image processing pipelines.
//! Rather than thread the rust state through the generated functions we just
//! store everything in globals...

use std::{collections::HashMap, ffi::CStr};
use libc::c_char;
use crate::tracing::*;

static mut TRACE_IDS: Option<HashMap<String, TraceId>> = None;
static mut TRACE: Option<Trace> = None;

/// Sets the global trace and trace id state.
pub unsafe fn set_global_trace(ids: HashMap<String, TraceId>, trace: Trace) {
    TRACE_IDS = Some(ids);
    TRACE = Some(trace);
}

/// Retrieves and clones the global trace state.
pub unsafe fn get_global_trace() -> Option<Trace> {
    if let Some(tr) = &TRACE { Some(tr.clone()) } else { None }
}

/// Clears the global trace and trace id state.
pub unsafe fn clear_global_trace() {
    TRACE_IDS = None;
    TRACE = None;
}

/// Records a read event in the global `TRACE`.
#[no_mangle]
pub extern "C" fn log_read(name: *const c_char, x: u32, y: u32) {
    unsafe {
        let name = CStr::from_ptr(name).to_string_lossy().to_string();
        if let (Some(tr), Some(ids)) = (&TRACE, &TRACE_IDS) {
            tr.trace_get(ids[&name], x as usize, y as usize);
        }
    }
}

/// Records a write event in the global `TRACE`.
#[no_mangle]
pub extern "C" fn log_write(name: *const c_char, x: u32, y: u32, c: u8) {
    unsafe {
        let name = CStr::from_ptr(name).to_string_lossy().to_string();
        if let (Some(tr), Some(ids)) = (&TRACE, &TRACE_IDS) {
            tr.trace_set(ids[&name], x as usize, y as usize, c);
        }
    }
}