mod fdu;
mod error;

use std::ffi::{CStr, CString};

// Provides a lot of C-equivalent types.
use libc::*;

// no_mangle tells Rust compiler not to mangle the name of the function and keep the original name.
//
// A flag starting with # is a macro. You can roughly think of it as a preprocessor directive, like
// @xxxx in Java/Kotlin/Dart and #xxxx in C.
#[no_mangle]
// The `pub` keyword makes the function public. Rust thinks it private if you don't use `pub`.
// The `extern "C"` keyword makes the function callable from C.
// The `->` indicates the function's return value type.
// The `*` means it is a raw, unsafe pointer, in the same way as * in C.
// The `mut` means the caller is able to change the value of pointer. To know more about mutability, see: https://doc.rust-lang.org/book/ch10-03-mutability.html
//
// Roughly speaking, `*mut c_char` is an equivalent of `char *` in C.
pub extern "C" fn hello_world() -> *mut c_char {
    // You can use CString::new to create a C-type String from a Rust String.
    // This is useful when you want to pass a String to a C function.
    // The CString will not be automatically freed by rust allocator, so you must do it manually in the C code.
    // p.s. The free functions for C are different in different platforms.
    //      For example, on Linux, it is free() and on Windows it is HeapFree().
    CString::new("hello world").unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    unsafe {
        if s.is_null() { return; }
        CString::from_raw(s)
    };
}

#[no_mangle]
pub extern "C" fn add(a: c_int, b: c_int) -> c_int {
    a + b
}

#[no_mangle]
pub extern "C" fn get_url(url: *const c_char) -> *mut c_char {
    // from_ptr() is an unsafe function! It operates a raw pointer and parses it into a &CStr.
    // So, you have to put it in an unsafe block. Otherwise, you will have to tag the whole function as unsafe.
    let c_str = unsafe {
        // A method ended with a ! is not a real method, it is also a macro.
        // Rust has a very advanced macro system. See the following link for more details:
        // https://doc.rust-lang.org/reference/macros.html
        assert!(!url.is_null());
        CStr::from_ptr(url)
    };
    // .to_str() will return an Option<&str>.
    //
    // &str is a reference to a str. A str is stored in the heap, and &str seems like a pointer to it.
    // You can never hold a str because str is a DST (dynamic sized type), which means we do not know
    // how much memory it will take. So you cannot hold a str in a variable, since variables are
    // always allocated on the stack, not heap. But hold its reference (&str) is okay.
    //
    // unwrap() is a method of Option<T>. You can view Option<T> as a nullable type T? in other languages.
    // The method will return the value inside the Option if it is Some(T),
    // or just panic if it is None. If a Rust program panics, the program will be terminated immediately.
    // So do not use unwrap() until you know what you are doing!
    //
    // There are many helper methods like unwrap(), such as expect(), unwrap_or(), unwrap_or_else() in std::option module.
    let url = c_str.to_str().unwrap();
    // Use blocking http client to get the content of the url.
    // Obviously, you cannot use async http client in the C code, so we drop any kind of async features in this project.
    let body = reqwest::blocking::get(url).unwrap().text().unwrap();
    CString::new(body).unwrap().into_raw()
}

// To prevent the linker breaking when the library is built without stdlib,
// we need to provide a dummy function.
//
// See it? The exclamation mark means that the function is diverging and will never return.
#[allow(dead_code)]
pub extern "C" fn fix_linking_when_not_using_stdlib() -> ! { panic!() }

// Test is an important part of the project.
// You can run all the tests by running `cargo test`.
//
// `#[cfg(test)]` is a macro that is used to conditionally compile code, the module with it is only compiled when the tests are run.
#[cfg(test)]
mod tests {
    // The `use` keyword is used to import a module.
    // The `super` keyword is used to refer to the parent module, i.e. the outer methods in this file,
    // such as `hello_world()`, `add()`.
    use super::*;

    // The `#[test]` macro is used to mark a function as a test.
    // All functions marked with `#[test]` will be run when you run `cargo test`.
    // Each test will be run in parallel in different threads, not one by one.
    #[test]
    fn it_works() {
        // Unsafe block is allowed in tests, but unsafe function is not.
        unsafe {
            // The `assert_eq!` macro is used to compare two values.
            // If the two values are not equal, the test will fail, and panic! will be called.
            assert_eq!(CString::from_raw(hello_world()), CString::new("hello world").unwrap());
            assert_eq!(add(1, 2), 3);
        }
    }
}
