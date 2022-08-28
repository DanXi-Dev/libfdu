use std::collections::HashMap;
use std::sync::Arc;
use std::{thread, time::Duration};

use reqwest::{header, redirect, Url};
use reqwest::blocking::{Client, ClientBuilder, RequestBuilder, Response};
use reqwest::cookie::{CookieStore, Jar};
use scraper::{Html, Selector};
use crate::error::SDKError;
use super::fdu_daily;
use crate::error::Error;

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
const LOGIN_SUCCESS_URL: &str = "https://uis.fudan.edu.cn/authserver/index.do";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML like Gecko) Chrome/91.0.4472.114 Safari/537.36";


// This is good practice to use a trait, only if you believe the same methods will be implemented for different structs.
// Otherwise, DO NOT bother yourself by declaring traits everywhere. Rust is sightly different from certain OOP languages, like Java,
// and it does not support polymorphism very well. It is hard to store different structs with same trait in a single list,
// you cannot store them on stack, and you can hardly decide their types at runtime because Rust is statically typed.
pub trait HttpClient {
    fn get_client(&self) -> &Client;

    fn client_builder() -> ClientBuilder {
        let mut headers = header::HeaderMap::new();
        headers.insert("Accept", header::HeaderValue::from_static("application/json;text/html;q=0.9,*/*;q=0.8"));
        headers.insert("Accept-Language", header::HeaderValue::from_static("zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7"));
        headers.insert("Cache-Control", header::HeaderValue::from_static("no-cache"));
        headers.insert("Connection", header::HeaderValue::from_static("keep-alive"));
        headers.insert("DNT", header::HeaderValue::from_static("1"));

        Client::builder()
            .cookie_store(true)
            .user_agent(UA)
            .default_headers(headers)
    }

    fn get_cookie_store(&self) -> &Arc<Jar>;

    // safely send a request from builder, dealing common errors
    // like repeat login and throttling
    fn send(&self, builder: RequestBuilder) -> Result<Response, reqwest::Error> {
        let req = builder.build()?;
        if let Some(mut request) = req.try_clone() {  // copy!
            let mut res = self.get_client().execute(req)?;
            // copy!
            let mut buf: Vec<u8> = vec![];
            res.copy_to(&mut buf)?;
            let html = String::from_utf8_lossy(&buf).to_string();

            // sleep for a while
            // will be throttled if duration is 1 second
            thread::sleep(Duration::from_millis(1500));

            if html.contains("当前用户存在重复登录的情况") {
                let document = Html::parse_document(html.as_str());
                for a in document.select(&Selector::parse("a").unwrap()){
                    if let Some(href) = a.value().attr("href"){
                        let url_ptr = request.url_mut();
                        *url_ptr = Url::parse(href).expect("");
                        println!("repeat login, redirect to {}", request.url().as_str());
                        return self.get_client().execute(request);
                    }
                }
            } else if html.contains("请不要过快点击") {
                return self.get_client().execute(request);
            }

            Ok(res)

        } else {
            return self.get_client().execute(req);
        }
    }
}

// Lesson time: why Rust needs explicit lifetime annotations?
//
// TL;DR: Rust compiler is indeed able to deduce the minial lifetime of return values, but it decides to leave
// the part for programmer to specify the lifetime and check it at build time.
//
// Imagine we have two functions:
// fn get_before(text: &str, sep: &str) -> &str; // Search for first [sep] in [text], and return the part before [sep].
// fn get_longest(a: &str, b: &str) -> &str; // Return the longest string between a and b.
//
// Naturally, we can see that:
// `get_longest` requires both [a] and [b] to live longer than the return value,
// while `get_before` requires just [text] to live longer than the return value.
//
// See? It is hard for somebody to guess a reasonable lifetime for return values. So you need to do it yourself:
// fn get_longest<'a>(a: &'a str, b: &'a str) -> &'a str;
// fn get_before<'a>(text: &'a str, sep: &str) -> &'a str;
//
// Then, compiler will check whether your lifetime is legal by running borrow checker in the function.
//
// You may argue: why the *smart* compiler does not do this for me? It can even check my declarations and data flows in the function!
//
// Part of the answer is: the lifetime is a part of function signature.
// Imagine that you decide to require the [sep] in `get_before` to live as long as [text] - no why, just your design. With specifying
// lifetime annotations, you can make sure the compiler will check your intent for you when compiling:
// fn get_before<'a>(text: &'a str, sep: &'a str) -> &'a str;
//
// But without such a feature, you cannot realize this great design. Because the compiler has hard-coded the lifetime of return values
// and parameters.
//
// Another part of the answer is: a compiler is not for deduction, but for check. Doing type inference is what it can do at most.
// An example is `mut`: if I ask you "why we need to write `mut` before variables we want to change? Why the compiler does not analyze my codes and
// decide whether a variable is mutable or not?", you may answer: "because we need to exactly know the mutability of a variable, or it is hard to read codes!"
// See, you have answered the question by yourself. The same answer is applicable to explicit lifetime annotations.
//
// What else: interestingly, there was once a RFC to call for automatic lifetime inference, but refused by Rust team.
// See https://github.com/rust-lang/rfcs/blob/master/text/2115-argument-lifetimes.md for details.
pub trait Account: HttpClient {
    fn set_credentials(&mut self, uid: &str, pwd: &str);

    fn login(&mut self, uid: &str, pwd: &str) -> Result<(), Error> {
        self.set_credentials(uid, pwd);

        let mut payload = HashMap::new();
        payload.insert("username", uid);
        payload.insert("password", pwd);

        // get some tokens
        let html = self.get_client().get(LOGIN_URL).send()?.text()?;
        let document = Html::parse_document(html.as_str());
        let selector = Selector::parse(r#"input[type="hidden"]"#).unwrap();
        for element in document.select(&selector) {
            let name = element.value().attr("name");
            if let Some(key) = name {
                payload.insert(key, element.value().attr("value").unwrap_or_default());
            }
        }

        // send login request
        let res = self.get_client().post(LOGIN_URL).form(&payload).send()?;

        // check if login is successful
        if res.url().as_str() == LOGIN_SUCCESS_URL {
            Ok(())
        } else {
            Err(Error::LoginError)
        }
    }

    fn logout(&self) -> Result<(), Error> {
        // TODO: logout service
        let res = self.get_client().get(LOGOUT_URL).query(&[("service", "")]).send()?;

        if res.status() != 200 {
            return Err(Error::LogoutError);
        }

        Ok(())
    }
}


pub struct Fdu {
    client: Client,
    cookie_store: Arc<Jar>,
    uid: Option<String>,
    pwd: Option<String>,
}

impl HttpClient for Fdu {
    fn get_client(&self) -> &Client {
        &self.client
    }

    fn get_cookie_store(&self) -> &Arc<Jar> {
        &self.cookie_store
    }
}

impl Account for Fdu {
    fn set_credentials(&mut self, uid: &str, pwd: &str) {
        self.uid = Some(uid.to_string());
        self.pwd = Some(pwd.to_string());
    }
}

impl Fdu {
    // It is always recommended to use `new()` to create an instance of a struct.
    pub(crate) fn new() -> Self {
        let cookie_store = Arc::new(Jar::default());
        let client = Self::client_builder()
            .cookie_provider(Arc::clone(&cookie_store))
            .build()
            .expect("client build failed");

        Self {
            client,
            cookie_store,
            uid: None,
            pwd: None,
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::fdu::jwfw::JwfwClient;
    use super::*;

    #[test]
    fn test_login_and_out() {
        dotenv::dotenv().ok();  // load env from .env file
        let uid = std::env::var("UID").expect("environment variable UID not set");
        let pwd = std::env::var("PWD").expect("environment variable PWD not set");

        let mut fd = Fdu::new();
        fd.login(uid.as_str(), pwd.as_str()).expect("login error");
        fd.logout().expect("logout error");
    }

    #[test]
    fn test_wrong_login() {
        let mut fd = Fdu::new();
        fd.login("123", "123").expect_err("expect error");
    }

    #[test]
    fn check_fdu_daily() {
        dotenv::dotenv().ok();
        let uid = std::env::var("UID").expect("environment variable UID not set");
        let pwd = std::env::var("PWD").expect("environment variable PWD not set");
        let mut fd = Fdu::new();
        fd.login(uid.as_str(), pwd.as_str()).expect("login error");
        fdu_daily::has_tick(&fd).unwrap();
    }
}