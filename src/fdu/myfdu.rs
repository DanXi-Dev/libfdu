use std::collections::HashMap;

use regex::Regex;
use scraper::{Html, Selector};

use crate::fdu::fdu::{Account, Fdu};

const MYFDU_URL: &str = "https://my.fudan.edu.cn/";
const COURSE_GRADE_URL: &str = "https://my.fudan.edu.cn/list/bks_xx_cj";

pub struct GradeData {
    id: String,
    name: String,
    academic_year: (i32, i32),
    semester: i32,
    credits: f32,
    grade: String,
}

impl MyFduClient for Fdu {}

pub trait MyFduClient: Account {
    fn get_myfdu_course_grade(&self) -> reqwest::Result<String>{
        let client = self.get_client();
        let html  = client.get(COURSE_GRADE_URL).send()?;
        println!("{:?}", html);
        Ok("das".to_string())
    }
}


#[cfg(test)]
mod tests {
    use crate::fdu::jwfw::JwfwClient;
    use super::*;

    #[test]
    fn test_myfdu() {
        dotenv::dotenv().ok();  // load env from .env file
        let uid = std::env::var("UID").expect("environment variable UID not set");
        let pwd = std::env::var("PWD").expect("environment variable PWD not set");

        let mut fd = Fdu::new();
        fd.login(uid.as_str(), pwd.as_str()).expect("login error");
        fd.get_myfdu_course_grade();
    }
}