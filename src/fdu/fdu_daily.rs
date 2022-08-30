use chrono::Local;
use serde_json::{Map, Value};
use super::fdu::*;
use crate::error::*;

const GET_INFO_URL: &str = "https://zlapp.fudan.edu.cn/ncov/wap/fudan/get-info";


pub fn get_history_info(fdu: &Fdu) -> Result<String> {
    Ok(fdu.get_client().get(GET_INFO_URL).send()?.text()?)
}

pub fn has_tick(fdu: &Fdu) -> Result<bool> {
    let history_json = get_history_info(fdu)?;
    let history: Value = serde_json::from_str(&history_json)?;

    // Stand for format error.
    let fe = |key: &str| SDKError::new(format!("Key {} is unable to be parsed in history json.", key));


    // 2022-08-28 (@w568w):
    // It is hard to use `?` to combine these functions: `.as_object().ok_or(fe("xxx"))` which returns `Result` and `.get("d")` which returns `Option`,
    // because Rust doesn't support mixed return types with `?`.
    // See https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html#where-the--operator-can-be-used.
    //
    // E.g. `obj.get_result()?.get_optional()?` is not allowed in any case.
    //
    // However, we DO need such a special `?`, which is able to use in the same function, and let:
    //
    // 1. `value?` stop the function and return `Err(e)` immediately if `value` is an `Err(e)`;
    // (1 is what Rust's `?` does now)
    // 2. `value?.foo()` equal `None` if `value` is `None`. This is the same function as the optional operator `?.` in JavaScript.
    // (2 is realized in Rust by `and_then`. But we cannot do 1 with `?` INSIDE `and_then`, because it's in a lambda-function and doesn't stop the function itself!)
    //
    // So I think we can only use `?`, `NoneError` and `try` here, and drop the convenience of `and_then`.
    // It makes the code less readable and hard to understand, but it is the only way to do it.

    let date: Result<&Value> = try {
        history.as_object().ok_or(fe("history"))?.get("d").ok_or(SDKError::none())?
            .as_object().ok_or(fe("d"))?.get("info").ok_or(SDKError::none())?
            .as_object().ok_or(fe("info"))?.get("date").ok_or(SDKError::none())?
    };
    if date.is_err() && !date.as_ref().unwrap_err().is_none_error() {
        return Err(date.unwrap_err());
    }

    // Or we can use `match` to do the same thing, but even less readable:
    // (Don't be afraid to use the same name for different type variables! It's just a convention.)
    //
    // let json = history.as_object().ok_or(fe("history"))?.get("d");
    // let json = match json {
    //     Some(d) => d.as_object().ok_or(fe("d"))?.get("info"),
    //     None => None,
    // };
    //
    // let json = match json {
    //     Some(info) => info.as_object().ok_or(fe("info"))?.get("date"),
    //     None => None
    // };
    if let Ok(date) = date {
        // get current time in yyyyMMdd
        Ok(Local::now().format("%Y%m%d").to_string() == date.as_str().unwrap_or_default())
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_fdu_daily() {
        dotenv::dotenv().ok();
        let uid = std::env::var("UID").expect("environment variable UID not set");
        let pwd = std::env::var("PWD").expect("environment variable PWD not set");
        let mut fd = Fdu::new();
        fd.login(uid.as_str(), pwd.as_str()).expect("login error");
        assert!(has_tick(&fd).unwrap())
    }
}