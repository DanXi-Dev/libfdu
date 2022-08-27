use std::ffi::{CStr, CString};
// Provides a lot of C-equivalent types.
use libc::*;

// no_mangle tells Rust compiler not to mangle the name of the function and keep the original name.
#[no_mangle]
pub extern "C" fn hello_world() -> *mut c_char {
    // You can use CString::new to create a C-type String from a Rust String.
    // This is useful when you want to pass a String to a C function.
    // The CString will not be automatically freed by rust allocator, so you must do it manually in the C code.
    CString::new("Hello Fudan!").unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn add(a: c_int, b: c_int) -> c_int {
    a + b
}

#[no_mangle]
pub extern "C" fn get_url(url: *const c_char) -> *mut c_char {
    let c_str = unsafe {
        assert!(!url.is_null());

        CStr::from_ptr(url)
    };
    let url = c_str.to_str().unwrap();
    // Use blocking http client to get the content of the url.
    // Obviously, you cannot use async http client in the C code.
    let body = reqwest::blocking::get(url).unwrap().text().unwrap();
    CString::new(body).unwrap().into_raw()
}

// To prevent the linker breaking when the library is built without stdlib,
// we need to provide a dummy function.
#[allow(dead_code)]
pub extern "C" fn fix_linking_when_not_using_stdlib() { panic!() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        unsafe {
            assert_eq!(CString::from_raw(hello_world()), CString::new("Hello Fudan!").unwrap());
            assert_eq!(add(1, 2), 3);
        }
    }
}
