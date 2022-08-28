use std::collections::HashMap;

use regex::Regex;
use scraper::{Html, Selector};

use crate::fdu::fdu::{Account, Fdu};

const JWFW_URL: &str = "https://jwfw.fudan.edu.cn/eams/home.action";
const JWFW_COURSE_TABLE_URL: &str = "https://jwfw.fudan.edu.cn/eams/courseTableForStd!courseTable.action";

impl JwfwClient for Fdu {}

// Parse the course time data from the javascript part of the raw html.
// The raw data for course time is like
/*
activity = new TaskActivity("155165","陈彤兵","42071(COMP130004.03)","数据结构(COMP130004.03)","320","HGX304","01111111111011111000000000000000000000000000000000000");
index =2*unitCount+0;
table0.activities[index][table0.activities[index].length]=activity;
index =2*unitCount+1;
table0.activities[index][table0.activities[index].length]=activity;
index =2*unitCount+2;
table0.activities[index][table0.activities[index].length]=activity;
activity = new TaskActivity("155165","陈彤兵","42071(COMP130004.03)","数据结构(COMP130004.03)","301","H3409","00000000000100000000000000000000000000000000000000000");
index =1*unitCount+7;
table0.activities[index][table0.activities[index].length]=activity;
index =1*unitCount+8;
table0.activities[index][table0.activities[index].length]=activity;
index =1*unitCount+9;
table0.activities[index][table0.activities[index].length]=activity;
 */
// the number in "index =2*unitCount+0;", "index =1*unitCount+8;", etc. implies the day and time for the course in the current week.
fn parse_course_time(html: &String) -> HashMap<String, Vec<(i32, i32)>> {
    let regexCourse = Regex::new(r##"activity = new TaskActivity\("\d+","\S+","\d+\((\w+.\w+)\)","\S+\(\w+.\w+\)","\d+","\w+","[01]+"\);((?:\s*index =\d+\*unitCount\+\d+;\s*table0.activities\[index\]\[table0.activities\[index\].length\]=activity;)+)"##).unwrap();
    let mut ret = HashMap::new();
    for capCourse in regexCourse.captures_iter(html.as_str()) {
        // Get the course code
        // e.g. "COMP130004.03"
        let courseCode = capCourse[1].to_string();
        // Get the data for each group, which is like
        /*
        index =2*unitCount+0;
        table0.activities[index][table0.activities[index].length]=activity;
        index =2*unitCount+1;
        table0.activities[index][table0.activities[index].length]=activity;
        index =2*unitCount+2;
        table0.activities[index][table0.activities[index].length]=activity;
        */
        let courseData = &capCourse[2];
        let regexLesson = Regex::new(r##"index =(\d+)\*unitCount\+(\d+);"##).unwrap();
        for capLesson in regexLesson.captures_iter(courseData) {
            let dayNumber: &i32 = &capLesson[1].parse().unwrap();
            let timeNumber: &i32 = &capLesson[2].parse().unwrap();
            match ret.get(&courseCode) {
                None => {
                    ret.insert(courseCode.to_string(), vec![(*dayNumber, *timeNumber)]);
                }
                Some(courseTime) => {
                    let mut courseTimeClone = courseTime.clone();
                    courseTimeClone.push((*dayNumber, *timeNumber));
                    ret.insert(courseCode.to_string(), courseTimeClone);
                }
            }
        }
    }
    println!("{:?}", ret);
    ret
}

pub trait JwfwClient: Account {
    fn get_jwfw_homepage(&self) -> reqwest::Result<String> {
        let client = self.get_client();
        let mut html = client.get(JWFW_URL).send()?.text()?;
        let document = Html::parse_document(html.as_str());
        let selector = Selector::parse(r#"html > body > a"#).unwrap();
        for element in document.select(&selector) {
            if element.inner_html().as_str() == "点击此处" {
                let href = element.value().attr("href");
                if let Some(key) = href {
                    html = client.get(key.to_string()).send()?.text()?
                }
            }
        }
        Ok(html)
    }

    fn get_course_table(&self) -> reqwest::Result<()> {
        let client = self.get_client();
        let mut payload = HashMap::new();
        payload.insert("ignoreHead", "1");
        payload.insert("setting.kind", "std");
        payload.insert("startWeek", "1");
        payload.insert("project.id", "1");
        payload.insert("semester.id", "385");
        payload.insert("ids", "403028");
        let html = client.post(JWFW_COURSE_TABLE_URL).form(&payload).send()?.text()?;
        println!("{}", html);
        parse_course_time(&html);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwfw() {
        dotenv::dotenv().ok();  // load env from .env file
        let uid = std::env::var("UID").expect("environment variable UID not set");
        let pwd = std::env::var("PWD").expect("environment variable PWD not set");

        let mut fd = Fdu::new();
        fd.login(uid.as_str(), pwd.as_str()).expect("login error");
        fd.get_jwfw_homepage().expect("jwfw error");
        fd.get_course_table().expect("jwfw course table error");
        fd.logout().expect("logout error");
    }
}
