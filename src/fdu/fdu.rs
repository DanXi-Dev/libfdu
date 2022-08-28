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