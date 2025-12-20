//! FFI Interface for LumaDB Core
//!
//! Provides C-compatible exports for integration with Go, Python, and other languages.
//! All functions are `extern "C"` and use raw pointers for interop.

use std::ffi::{c_char, c_void, CStr, CString};
use std::slice;
use std::ptr;

/// Result codes
pub const LUMA_OK: i32 = 0;
pub const LUMA_ERR_INVALID_ARG: i32 = -1;
pub const LUMA_ERR_INTERNAL: i32 = -2;
pub const LUMA_ERR_NOT_FOUND: i32 = -3;

/// Opaque handle to LumaDB engine
pub struct LumaEngineHandle {
    // In real impl: contains ShardCoordinator, Config, etc.
}

/// Initialize LumaDB engine from JSON config
#[no_mangle]
pub extern "C" fn lumadb_init(config_json: *const c_char) -> *mut c_void {
    if config_json.is_null() {
        return ptr::null_mut();
    }

    let _config_str = match unsafe { CStr::from_ptr(config_json).to_str() } {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    // In real impl: parse config, create ShardCoordinator
    let handle = Box::new(LumaEngineHandle {});
    Box::into_raw(handle) as *mut c_void
}

/// Shutdown engine and free resources
#[no_mangle]
pub extern "C" fn lumadb_shutdown(engine: *mut c_void) -> i32 {
    if engine.is_null() {
        return LUMA_ERR_INVALID_ARG;
    }

    unsafe {
        let _ = Box::from_raw(engine as *mut LumaEngineHandle);
    }

    LUMA_OK
}

/// Execute a serialized query plan
/// 
/// # Safety
/// Caller must ensure plan_ptr is valid for plan_len bytes.
/// Caller must call lumadb_free_result on the returned result.
#[no_mangle]
pub unsafe extern "C" fn lumadb_execute(
    engine: *mut c_void,
    plan_ptr: *const u8,
    plan_len: usize,
    result_ptr: *mut *mut u8,
    result_len: *mut usize,
) -> i32 {
    if engine.is_null() || plan_ptr.is_null() || result_ptr.is_null() || result_len.is_null() {
        return LUMA_ERR_INVALID_ARG;
    }

    let _engine = &*(engine as *const LumaEngineHandle);
    let _plan_bytes = slice::from_raw_parts(plan_ptr, plan_len);

    // In real impl: deserialize plan, execute, serialize result
    let result = b"{}".to_vec();
    let boxed = result.into_boxed_slice();
    *result_len = boxed.len();
    *result_ptr = Box::into_raw(boxed) as *mut u8;

    LUMA_OK
}

/// Free a result buffer allocated by lumadb_execute
#[no_mangle]
pub unsafe extern "C" fn lumadb_free_result(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        let _ = Box::from_raw(slice::from_raw_parts_mut(ptr, len) as *mut [u8]);
    }
}

/// Get engine metrics as JSON string
/// 
/// # Safety
/// Caller must call lumadb_free_string on the returned pointer.
#[no_mangle]
pub extern "C" fn lumadb_metrics(engine: *mut c_void) -> *mut c_char {
    if engine.is_null() {
        return ptr::null_mut();
    }

    let metrics_json = r#"{"ops_count": 0, "latency_p99_us": 0}"#;
    match CString::new(metrics_json) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Free a string allocated by LumaDB
#[no_mangle]
pub unsafe extern "C" fn lumadb_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

// ============ SIMD Exports for Go FFI ============

/// SIMD-accelerated sum of i64 array
#[no_mangle]
pub unsafe extern "C" fn lumadb_simd_sum_i64(data: *const i64, len: usize) -> i64 {
    if data.is_null() || len == 0 {
        return 0;
    }

    let slice = slice::from_raw_parts(data, len);
    crate::simd::SimdDispatcher::detect().sum_i64(slice)
}

/// SIMD-accelerated sum of f64 array
#[no_mangle]
pub unsafe extern "C" fn lumadb_simd_sum_f64(data: *const f64, len: usize) -> f64 {
    if data.is_null() || len == 0 {
        return 0.0;
    }

    let slice = slice::from_raw_parts(data, len);
    crate::simd::SimdDispatcher::detect().sum_f64(slice)
}

/// SIMD-accelerated min of i64 array
#[no_mangle]
pub unsafe extern "C" fn lumadb_simd_min_i64(data: *const i64, len: usize) -> i64 {
    if data.is_null() || len == 0 {
        return i64::MAX;
    }

    let slice = slice::from_raw_parts(data, len);
    crate::simd::SimdDispatcher::detect().min_i64(slice).unwrap_or(i64::MAX)
}

/// SIMD-accelerated max of i64 array
#[no_mangle]
pub unsafe extern "C" fn lumadb_simd_max_i64(data: *const i64, len: usize) -> i64 {
    if data.is_null() || len == 0 {
        return i64::MIN;
    }

    let slice = slice::from_raw_parts(data, len);
    crate::simd::SimdDispatcher::detect().max_i64(slice).unwrap_or(i64::MIN)
}

// ============ Version ============

/// Get LumaDB version string
#[no_mangle]
pub extern "C" fn lumadb_version() -> *const c_char {
    static VERSION: &str = "3.0.0-world-class\0";
    VERSION.as_ptr() as *const c_char
}
