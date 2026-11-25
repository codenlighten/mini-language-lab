//! WebAssembly entry points with a plain C ABI, so the browser needs no generated glue code.
//!
//! Protocol (single-threaded): JavaScript calls `alloc(n)`, writes `n` UTF-8 bytes there, calls
//! `analyze(ptr, n)` or `format_source(ptr, n)`, reads the result from `output_ptr()` for the returned
//! length, then calls `dealloc(ptr, n)` for the input. The output buffer is reused by the next call.

use std::cell::RefCell;

thread_local! {
    static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Reserves `len` bytes for the caller to fill. Release them with [`dealloc`] using the same length.
#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    Box::into_raw(vec![0u8; len].into_boxed_slice()) as *mut u8
}

/// # Safety
/// `ptr` and `len` must come from one earlier call to [`alloc`], and must not be released twice.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
}

fn set_output(bytes: Vec<u8>) -> usize {
    let n = bytes.len();
    OUTPUT.with(|o| *o.borrow_mut() = bytes);
    n
}

/// Pointer to the bytes written by the last call.
#[no_mangle]
pub extern "C" fn output_ptr() -> *const u8 {
    OUTPUT.with(|o| o.borrow().as_ptr())
}

/// # Safety
/// `ptr` must be valid for reads of `len` bytes for as long as the returned text is used.
unsafe fn read_input<'a>(ptr: *const u8, len: usize) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    let bytes = std::slice::from_raw_parts(ptr, len);
    std::str::from_utf8(bytes).ok()
}

/// Analyzes the program and returns the length of the JSON result, or `-1` if the input is not UTF-8.
///
/// # Safety
/// `ptr` must point to `len` readable bytes (a region from [`alloc`] that the caller filled).
#[no_mangle]
pub unsafe extern "C" fn analyze(ptr: *const u8, len: usize) -> isize {
    match read_input(ptr, len) {
        Some(text) => set_output(
            procflow_core::analysis_json(text, &procflow_core::analyze(text)).into_bytes(),
        ) as isize,
        None => -1,
    }
}

/// Formats the program and returns the length of the result, `-1` if the input is not UTF-8, or `-2`
/// if the program has syntax errors and so cannot be formatted.
///
/// # Safety
/// `ptr` must point to `len` readable bytes (a region from [`alloc`] that the caller filled).
#[no_mangle]
pub unsafe extern "C" fn format_source(ptr: *const u8, len: usize) -> isize {
    match read_input(ptr, len) {
        Some(text) => match procflow_core::format(text) {
            Some(out) => set_output(out.into_bytes()) as isize,
            None => -2,
        },
        None => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(f: unsafe extern "C" fn(*const u8, usize) -> isize, input: &[u8]) -> (isize, String) {
        let p = alloc(input.len());
        unsafe { std::ptr::copy_nonoverlapping(input.as_ptr(), p, input.len()) };
        let n = unsafe { f(p, input.len()) };
        let out = if n >= 0 {
            unsafe { std::slice::from_raw_parts(output_ptr(), n as usize) }.to_vec()
        } else {
            Vec::new()
        };
        unsafe { dealloc(p, input.len()) };
        (n, String::from_utf8(out).unwrap())
    }

    #[test]
    fn analyze_returns_exactly_the_cores_json_through_the_buffer() {
        let text = "process P { start -> a\n end a }";
        let (n, json) = call(analyze, text.as_bytes());
        // Compared with what the core produces directly, so a wrong length or a wrong byte fails.
        assert_eq!(
            json,
            procflow_core::analysis_json(text, &procflow_core::analyze(text))
        );
        assert_eq!(n as usize, json.len());
        assert!(json.ends_with('}'));
    }

    #[test]
    fn invalid_utf8_is_refused_not_a_crash() {
        assert_eq!(call(analyze, &[0xff, 0xfe, 0xfd]).0, -1);
        assert_eq!(call(format_source, &[0xc0, 0x80]).0, -1);
    }

    #[test]
    fn format_returns_text_or_a_clear_code() {
        let (n, out) = call(format_source, b"process P{start->a end a}");
        assert_eq!(
            out,
            procflow_core::format("process P{start->a end a}").unwrap()
        );
        assert_eq!(n as usize, out.len());
        assert!(out.ends_with("}\n"), "{out}");
        assert_eq!(call(format_source, b"process P { start a }").0, -2);
    }

    #[test]
    fn empty_input_and_repeated_calls_reuse_the_buffer_safely() {
        assert!(call(analyze, b"").0 > 0);
        let (_, a) = call(analyze, b"process P { start -> x\n end x }");
        let (_, b) = call(analyze, b"process P { start -> x\n end x }");
        assert_eq!(a, b);
    }
}
