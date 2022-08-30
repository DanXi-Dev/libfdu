use std::collections::HashMap;

use regex::Regex;
use scraper::{Html, Selector};

use crate::fdu::fdu::{Account, Fdu};

const JWFW_URL: &str = "https://jwfw.fudan.edu.cn/eams/home.action";
const JWFW_COURSE_TABLE_QUERY_URL: &str = "https://jwfw.fudan.edu.cn/eams/courseTableForStd!courseTable.action";
const JWFW_COURSE_TABLE_MAIN_URL: &str = "https://jwfw.fudan.edu.cn/eams/courseTableForStd.action";

impl JwfwClient for Fdu {}

// Parse the ids(a value related to student id) from courseTableForStd.action
fn parse_ids(html: &String) -> String {
    let regex = Regex::new(r##"bg.form.addInput\(form,"ids","(\d+)"\);"##).unwrap();
    let cap = regex.captures_iter(html).next().unwrap();
    cap[1].to_string()
}

#[derive(Debug)]
pub struct CourseData {
    id: String,
    teacher: String,
    name_with_course_id: String,
    classroom: String,
    weeks: Vec<i32>,
    time: Vec<(i32, i32)>,
}

// Parse the course data from the javascript part of the raw html.
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
fn parse_course_data(html: &String) -> Vec<CourseData> {
    let regex_course = Regex::new(r##"activity = new TaskActivity\("(\d+)","(\S+)","\d+\(\w+.\w+\)","(\S+\(\w+.\w+\))","\d+","(\S+)","([01]+)"\);((?:\s*index =\d+\*unitCount\+\d+;\s*table0.activities\[index]\[table0.activities\[index].length]=activity;)+)"##).unwrap();
    let mut ret = Vec::new();
    for cap_course in regex_course.captures_iter(html.as_str()) {

        // Get the week info for the course
        // e.g. "01111111111011111000000000000000000000000000000000000"
        // The position with value 1 means there's a lesson in the week of its index.
        let course_week_info = cap_course[5].to_string();

        // Convert the week info to vector.
        // e.g. "01111111111011111000000000000000000000000000000000000" converts to vec![1,2,3,4,5,6,7,8,9,10,12,13,14,15,16]
        let mut weeks: Vec<i32> = Vec::new();
        for (i, c) in course_week_info.chars().enumerate() {
            if c == '1' {
                weeks.push(i as i32);
            }
        }


        // Get the data for each group, which is like
        /*
        index =2*unitCount+0;
        table0.activities[index][table0.activities
        [index].length]=activity;
        index =2*unitCount+1;
        table0.activities[index][table0.activities[index].length]=activity;
        index =2*unitCount+2;
        table0.activities[index][table0.activities[index].length]=activity;
        */

        let mut time: Vec<(i32, i32)> = Vec::new();
        let course_data = &cap_course[6];
        let regex_lesson = Regex::new(r##"index =(\d+)\*unitCount\+(\d+);"##).unwrap();
        for cap_lesson in regex_lesson.captures_iter(course_data) {
            let day_number: &i32 = &cap_lesson[1].parse().unwrap();
            let time_number: &i32 = &cap_lesson[2].parse().unwrap();
            time.push((*day_number, *time_number));
        }
        ret.push(CourseData {
            id: cap_course[1].to_string(),
            teacher: cap_course[2].to_string(),
            name_with_course_id: cap_course[3].to_string(),
            classroom: cap_course[4].to_string(),
            weeks,
            time,
        })
    }
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

    fn get_course_table(&self) -> reqwest::Result<Vec<CourseData>> {
        let client = self.get_client();

        // First visit the courseTableForStd.action to get ids(a value related to student id)
        let main_html = client.get(JWFW_COURSE_TABLE_MAIN_URL).send()?.text()?;
        let ids = parse_ids(&main_html);

        let mut payload = HashMap::new();
        payload.insert("ignoreHead", "1");
        payload.insert("setting.kind", "std");
        payload.insert("startWeek", "1");
        payload.insert("project.id", "1");
        payload.insert("semester.id", "385");
        payload.insert("ids", ids.as_str());
        let query_html = client.post(JWFW_COURSE_TABLE_QUERY_URL).form(&payload).send()?.text()?;
        let course_data = parse_course_data(&query_html);
        println!("{:#?}", course_data);
        panic!("");
        Ok(course_data)
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
