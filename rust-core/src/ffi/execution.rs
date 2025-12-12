use crate::execution::join::HashJoiner;
use libc::{c_char, c_void};
use std::ffi::{CStr, CString};
use serde_json::Value;

/// FFI export for Hash Join.
/// 
/// Accepts partial JSON arrays as strings for left and right relations.
/// Returns a JSON string of the joined result.
/// 
/// Caller must free expectation result with `luma_free_string` (already in ffi/utils or similar, or we add one).
#[no_mangle]
pub extern "C" fn luma_execution_hash_join(
    left_json: *const c_char,
    right_json: *const c_char,
    key: *const c_char,
) -> *mut c_char {
    // Safety: Pointers must be valid C strings
    let left_str = unsafe {
        if left_json.is_null() { return std::ptr::null_mut(); }
        CStr::from_ptr(left_json).to_string_lossy()
    };
    let right_str = unsafe {
        if right_json.is_null() { return std::ptr::null_mut(); }
        CStr::from_ptr(right_json).to_string_lossy()
    };
    let key_str = unsafe {
        if key.is_null() { return std::ptr::null_mut(); }
        CStr::from_ptr(key).to_string_lossy()
    };

    // Deserialize
    let left_vec: Vec<Value> = match serde_json::from_str(&left_str) {
        Ok(v) => v,
        Err(_) => return std::ptr::null_mut(), // Error handling MVP: return null
    };
    let right_vec: Vec<Value> = match serde_json::from_str(&right_str) {
        Ok(v) => v,
        Err(_) => return std::ptr::null_mut(),
    };

    // Execute
    let result = HashJoiner::execute(left_vec, right_vec, &key_str);

    // Serialize
    let res_str = match serde_json::to_string(&result) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    // Return as C String
    match CString::new(res_str) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

use crate::execution::aggregate::Aggregator;

#[no_mangle]
pub extern "C" fn luma_execution_aggregate(
    values_json: *const c_char,
    op: *const c_char,
) -> f64 {
    if values_json.is_null() || op.is_null() {
        return 0.0;
    }

    let c_str_vals = unsafe { CStr::from_ptr(values_json) };
    let values_str = match c_str_vals.to_str() {
        Ok(s) => s,
        Err(_) => return 0.0,
    };

    let values: Vec<Value> = match serde_json::from_str(values_str) {
        Ok(v) => v,
        Err(_) => return 0.0,
    };

    let c_str_op = unsafe { CStr::from_ptr(op) };
    let op_str = match c_str_op.to_str() {
        Ok(s) => s,
        Err(_) => return 0.0,
    };

    match op_str {
        "SUM" => Aggregator::sum(values),
        "AVG" => Aggregator::avg(values),
        "MIN" => Aggregator::min(values),
        "MAX" => Aggregator::max(values),
        "MAX" => Aggregator::max(values),
         _ => 0.0,
    }
}

/// Frees a C String allocated by Rust
#[no_mangle]
pub extern "C" fn luma_free_string(s: *mut c_char) {
    if s.is_null() { return; }
    unsafe {
        let _ = CString::from_raw(s);
    }
}
