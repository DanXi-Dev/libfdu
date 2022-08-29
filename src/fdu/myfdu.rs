use std::collections::HashMap;

use regex::Regex;
use scraper::{Html, Selector};

use crate::fdu::fdu::{Account, Fdu};

const MYFDU_URL: &str = "https://my.fudan.edu.cn/";
const COURSE_GRADE_URL: &str = "https://my.fudan.edu.cn/list/bks_xx_cj";
#[derive(Debug)]
pub struct GradeData {
    id: String,
    name: String,
    academic_year: String,
    semester: String,
    credits: f32,
    grade: String,
}

impl MyFduClient for Fdu {}

pub trait MyFduClient: Account {
    fn get_myfdu_course_grade(&self) -> reqwest::Result<Vec<GradeData>> {
        let client = self.get_client();
        let html = client.get(COURSE_GRADE_URL).send()?.text()?;
        let document = Html::parse_document(html.as_str());
        let selector = Selector::parse("#dataTable_BksXxCj>tbody>tr").unwrap();
        let mut grade_data: Vec<GradeData> = Vec::new();
        for element in document.select(&selector) {
            let sub_selector = Selector::parse("td").unwrap();
            let mut sub_element = element.select(&sub_selector);
            let course_info: GradeData = GradeData {
                id: sub_element.next().unwrap().inner_html(),
                academic_year: sub_element.next().unwrap().inner_html(),
                semester: sub_element.next().unwrap().inner_html(),
                name: sub_element.next().unwrap().inner_html(),
                credits: sub_element.next().unwrap().inner_html().parse().unwrap(),
                grade: sub_element.next().unwrap().inner_html(),
            };
            // println!("{:?}", course_info);
            grade_data.push(course_info);
        }
        Ok(grade_data)
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
        fd.get_myfdu_course_grade().expect("my fdu error");
        fd.logout().expect("logout error");
    }
}