use std::collections::HashMap;
use std::env;

use reqwest::{header, redirect};
use reqwest::blocking::Client;
use scraper::{Html, Selector};

// `const` declares a constant, which will be replaced with its value during compilation.
//
// Then what about a global variable? Well, this problem is much more complex.
// A global variable that:
// - get its value before start, and will not change: why not use `const` directly?
// - get its initial value at runtime, and will not change: use lazy_static! macro from lazy_static crate; or just use `OnceCell<T>` from some crates.
// - get its value before start, and will change, and the variable is small: use `static` keyword and `Cell<T>` type.
// - get its value before start, and will change, and the variable is large: use `static` keyword and `RefCell<T>` type. Note: `RefCell<T>` is unsafe and can panic!
// - get its initial value at runtime, and will change: same to above, use a XXCell<Option<T>> type and set it to None at the beginning.
// - get its value *whenever*, and will change, and I care about thread safety so much: use `static` keyword and `RwLock<T>` type.
//
// Even though you can declare a global variable with `static mut` keyword, it is unsafe and not recommended.
const LOGIN_URL: &str = "https://uis.fudan.edu.cn/authserver/login";
const LOGOUT_URL: &str = "https://uis.fudan.edu.cn/authserver/logout";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML like Gecko) Chrome/91.0.4472.114 Safari/537.36";

// This is good practice to use a trait, only if you believe the same methods will be implemented for different structs.
// Otherwise, DO NOT bother yourself by declaring traits everywhere. Rust is sightly different from certain OOP languages, like Java,
// and it does not support polymorphism very well. It is hard to store different structs with same trait in a single list,
// you cannot store them on stack, and you can hardly decide their types at runtime because Rust is statically typed.
trait FduInterface {
    fn login(&self, uid: &str, pwd: &str) -> Result<(), reqwest::Error>;
    fn logout(&self) -> Result<(), reqwest::Error>;
}

struct Fdu {
    pub client: Client,
}

impl Fdu {
    // It is always recommended to use `new()` to create an instance of a struct.
    fn new() -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert("Accept", header::HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9"));
        headers.insert("Accept-Language", header::HeaderValue::from_static("zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7"));
        headers.insert("Cache-Control", header::HeaderValue::from_static("no-cache"));
        headers.insert("Connection", header::HeaderValue::from_static("keep-alive"));
        headers.insert("DNT", header::HeaderValue::from_static("1"));

        let client = Client::builder()
            .cookie_store(true)
            .redirect(redirect::Policy::none())
            .user_agent(UA)
            .default_headers(headers)
            .build()
            .expect("client build failed");

        Self {
            client,
        }
    }
}

impl FduInterface for Fdu {
    fn login(&self, uid: &str, pwd: &str) -> Result<(), reqwest::Error> {
        let mut payload = HashMap::new();
        payload.insert("username", uid);
        payload.insert("password", pwd);

        // get some tokens
        let html = self.client.get(LOGIN_URL).send()?.text()?;
        let document = Html::parse_document(html.as_str());
        let selector = Selector::parse(r#"input[type="hidden"]"#).unwrap();
        for element in document.select(&selector) {
            let name = element.value().attr("name");
            if let Some(key) = name {
                payload.insert(key, element.value().attr("value").unwrap_or_default());
            }
        }

        // send login request
        let res = self.client.post(LOGIN_URL).form(&payload).send()?;

        if res.status() != 302 {
            // TODO: custom error
            panic!("login error");
        }

        Ok(())
    }

    fn logout(&self) -> Result<(), reqwest::Error> {
        // TODO: logout service
        let res = self.client.get(LOGOUT_URL).query(&[("service", "")]).send()?;

        if res.status() != 302 {
            // TODO: custom error
            panic!("logout error");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_and_out() {
        let uid = env::var("UID").expect("environment variable UID not set");
        let pwd = env::var("PWD").expect("environment variable PWD not set");

        let fd = Fdu::new();
        fd.login(uid.as_str(), pwd.as_str()).expect("login error");
        fd.logout().expect("logout error");
    }
}