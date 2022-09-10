use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use regex::Regex;
use reqwest::blocking::Client;
use reqwest::cookie::Jar;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

use crate::error::{ErrorType, Result, SDKError};

use super::fdu::*;

struct XK {
    fdu: Fdu,
    profile_id: i32,
    courses: Vec<Course>,
}

impl XK {
    fn new() -> Self {
        Self {
            fdu: Fdu::new(),
            profile_id: 0,
            courses: Vec::new(),
        }
    }

    fn new_from_fdu(fdu: Fdu) -> Self {
        Self {
            fdu,
            profile_id: 0,
            courses: Vec::new(),
        }
    }
}

impl HttpClient for XK {
    fn get_client(&self) -> &Client {
        &self.fdu.get_client()
    }

    fn get_cookie_store(&self) -> &Arc<Jar> {
        &self.fdu.get_cookie_store()
    }
}

impl Account for XK {
    fn set_credentials(&mut self, uid: &str, pwd: &str) {
        &self.fdu.set_credentials(uid, pwd);
    }

    fn login(&mut self, uid: &str, pwd: &str) -> Result<()> {
        self.set_credentials(uid, pwd);

        const LOGIN_URL: &str = "https://xk.fudan.edu.cn/xk/login.action";
        const LOGIN_SUCCESS_URL: &str = "https://xk.fudan.edu.cn/xk/home.action";

        // login
        let mut payload = HashMap::new();
        payload.insert("username", uid);
        payload.insert("password", pwd);
        let res = self.get_client().post(LOGIN_URL).form(&payload).send()?;
        if !res.url().as_str().starts_with(LOGIN_SUCCESS_URL) {
            return Err(SDKError::with_type(ErrorType::LoginError, "login error".to_string()));
        }

        // sleep
        thread::sleep(Duration::from_millis(1500));

        // get profile id
        const XK_URL: &str = "https://xk.fudan.edu.cn/xk/stdElectCourse!defaultPage.action";
        let html = self.get_client().get(XK_URL).send()?.text()?;
        let document = Html::parse_document(html.as_str());
        let selector = Selector::parse(r#"input[type="hidden"]"#).unwrap();
        if let Some(element) = document.select(&selector).next() {
            self.profile_id = element.value().attr("value").unwrap_or_default().parse::<i32>().unwrap_or_default();
            if self.profile_id == 0 {
                return Err(SDKError::with_type(ErrorType::ParseError, "get profile id error".to_string()));
            }
        } else {
            return Err(SDKError::with_type(ErrorType::ParseError, "get profile id error".to_string()));
        }

        // sleep
        thread::sleep(Duration::from_millis(1500));

        // access XK_URL otherwise we couldn't get courses
        let mut payload = HashMap::new();
        payload.insert("electionProfile.id", self.profile_id);
        let res = self.get_client().post(XK_URL).form(&payload).send()?;
        if res.status() != 200 {
            return Err(SDKError::with_type(ErrorType::LoginError, "access xk page error".to_string()));
        }
        Ok(())
    }

    fn logout(&self) -> Result<()> {
        const LOGOUT_URL: &str = "https://xk.fudan.edu.cn/xk/logout.action";
        let res = self.get_client().get(LOGOUT_URL).send()?;
        if res.status() != 200 {
            return Err(SDKError::with_type(ErrorType::LoginError, "logout failed".to_string()));
        }
        Ok(())
    }
}

#[derive(Serialize, Default)]
struct CourseQuery {
    #[serde(rename = "lessonNo")]
    no: String,
    // eg. ECON130213.01
    #[serde(rename = "courseCode")]
    code: String,
    // eg. ECON130213
    #[serde(rename = "courseName")]
    name: String, // eg. 计量经济学
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Course {
    id: i32,
    // eg. 123456
    no: String,
    // eg. ECON130213.01
    code: String,
    // eg. ECON130213
    name: String,
    // eg. 计量经济学
    #[serde(default)]
    amount: AmountInfo,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct AmountInfo {
    #[serde(rename = "lc")]
    total: i32,
    #[serde(rename = "sc")]
    selected: i32,
}

impl XK {
    fn query_course(&self, query: &CourseQuery) -> Result<Vec<Course>> {
        const QUERY_COURSE_URL: &str = "https://xk.fudan.edu.cn/xk/stdElectCourse!queryLesson.action";
        let res = self.get_client().
            post(QUERY_COURSE_URL).
            query(&[("profileId", self.profile_id)]).
            form(query).
            send()?;
        let status_code = res.status();
        let html = res.text()?;
        if status_code != 200 {
            return Err(SDKError::with_type(ErrorType::NetworkError, format!("status code: {}\ntext: {}", status_code, html)));
        }

        let r = Regex::new(r"(\[.+])[\s\S]*?(\{.+})").unwrap();
        let cap = r.captures(html.as_str()).ok_or(SDKError::with_type(ErrorType::ParseError, "parse course error".to_string()))?;
        let courses_str = normalize_json(
            cap.get(1).ok_or(SDKError::with_type(ErrorType::ParseError, "course_str does not exist".to_string()))?.as_str()
        );
        let amounts_str = normalize_json(
            cap.get(2).ok_or(SDKError::with_type(ErrorType::ParseError, "amounts_str does not exist".to_string()))?.as_str()
        );

        let mut courses: Vec<Course> = serde_json::from_str(courses_str.as_str())?;
        let amounts: HashMap<&str, AmountInfo> = serde_json::from_str(amounts_str.as_str())?;
        for course in &mut courses {
            if let Some(amount) = amounts.get(course.id.to_string().as_str()) {
                course.amount = amount.clone();
            }
        }
        Ok(courses)
    }

    fn get_courses(&mut self) -> Result<Vec<Course>> {
        if self.courses.len() > 0 {
            return Ok(self.courses.clone());
        }
        let courses = self.query_course(&CourseQuery::default())?;
        self.courses = courses;
        Ok(self.courses.clone())
    }

    fn get_id(&mut self, query: &CourseQuery, courses: Vec<Course>) -> Result<i32> {
        for course in courses {
            if course.no == query.no || course.code == query.code || course.name == query.name {
                return Ok(course.id);
            }
        }
        Err(SDKError::with_type(ErrorType::OtherError, "id not found".to_string()))
    }

    fn operate_course(&self, id: i32, select: bool) -> Result<bool> {
        // select: true -> select, false -> drop

        const OPERATE_COURSE_URL: &str = "https://xk.fudan.edu.cn/xk/stdElectCourse!batchOperator.action";
        let mut payload = HashMap::new();
        let mut operator0 = String::new();
        if select {
            payload.insert("optype", "true");
            operator0 = format!("{}:true:0", id);
        } else {
            payload.insert("optype", "false");
            operator0 = format!("{}:false", id);
        }
        payload.insert("operator0", operator0.as_str());

        let mut html = self.get_client().
            post(OPERATE_COURSE_URL).
            query(&[("profileId", self.profile_id)]).
            form(&payload).
            send()?.text()?;

        let document = Html::parse_document(html.as_str());
        let selector = Selector::parse("div").unwrap();
        html = document.select(&selector).next().
            ok_or(SDKError::with_type(ErrorType::ParseError, "operate course result".to_string()))?.
            text().collect();
        html.retain(|c| !c.is_whitespace());

        println!("{}", html);
        Ok(html.contains("成功"))
    }

    fn single_select(&mut self, query: &CourseQuery, select: bool) -> Result<bool> {
        let courses = self.query_course(query)?;
        let id = self.get_id(query, courses)?;
        self.operate_course(id, select)
    }
}

fn normalize_json(json: &str) -> String {
    let r1 = Regex::new(r"([a-zA-Z]+?):").unwrap();
    let mut result = r1.replace_all(json, "\"${1}\":").to_string();
    result = result.replace("'", "\"");
    result
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

        let mut xk = XK::new();
        xk.login(uid.as_str(), pwd.as_str()).expect("login error");
        xk.logout().expect("logout error");
    }

    #[test]
    fn test_get_course() {
        dotenv::dotenv().ok();  // load env from .env file
        let uid = std::env::var("UID").expect("environment variable UID not set");
        let pwd = std::env::var("PWD").expect("environment variable PWD not set");

        let mut xk = XK::new();
        xk.login(uid.as_str(), pwd.as_str()).expect("login error");

        let courses = xk.get_courses().expect("query course error");
        println!("{:?}", courses);

        xk.logout().expect("logout error");
    }

    #[test]
    fn test_select() {
        dotenv::dotenv().ok();  // load env from .env file
        let uid = std::env::var("UID").expect("environment variable UID not set");
        let pwd = std::env::var("PWD").expect("environment variable PWD not set");

        let mut xk = XK::new();
        xk.login(uid.as_str(), pwd.as_str()).expect("login error");

        let query = CourseQuery {
            name: "中国史前考古".to_string(),
            ..Default::default()
        };
        xk.single_select(&query, true).expect("select course error");
        thread::sleep(Duration::from_millis(1500));
        xk.single_select(&query, false).expect("select course error");

        xk.logout().expect("logout error");
    }

    #[test]
    fn test_normalize_json() {
        const COURSE: &str = "[{id:698241,no:'ECON130003.01',name:'国际金融',teachDepartName:'经济学院',code:'ECON130003',credits:3.0,courseId:38081,examTime:'2022-12-27 08:30-10:30 第17周 星期二',examFormName:'闭卷',startWeek:1,endWeek:16,courseTypeId:7,courseTypeName:'专业必修课程',courseTypeCode:'03_01',scheduled:true,hasTextBook:false,period:54,weekHour:3.0,withdrawable:true,textbooks:'',teachers:'郑辉',campusCode:'H',campusName:'邯郸校区',remark:'',arrangeInfo:[{weekDay:2,weekState:'01111111111111111000000000000000000000000000000000000',startUnit:3,endUnit:5,weekStateDigest:'1-16',rooms:'H3208'}]},{id:698246,no:'ECON130004.02',name:'国际贸易',teachDepartName:'经济学院',code:'ECON130004',credits:3.0,courseId:38082,examTime:'2023-01-03 13:00-15:00 第18周 星期二',examFormName:'闭卷',startWeek:1,endWeek:16,courseTypeId:7,courseTypeName:'专业必修课程',courseTypeCode:'03_01',scheduled:true,hasTextBook:false,period:54,weekHour:3.0,withdrawable:true,textbooks:'',teachers:'程大中',campusCode:'H',campusName:'邯郸校区',remark:'',arrangeInfo:[{weekDay:1,weekState:'01111111111111111000000000000000000000000000000000000',startUnit:3,endUnit:5,weekStateDigest:'1-16',rooms:'H5102'}]},{id:698257,no:'ECON130022.01',name:'货币经济学',teachDepartName:'经济学院',code:'ECON130022',credits:3.0,courseId:38100,examTime:'2022-12-29 08:30-10:30 第17周 星期四',examFormName:'闭卷',startWeek:1,endWeek:16,courseTypeId:7,courseTypeName:'专业必修课程',courseTypeCode:'03_01',scheduled:true,hasTextBook:false,period:54,weekHour:3.0,withdrawable:true,textbooks:'',teachers:'田素华',campusCode:'H',campusName:'邯郸校区',remark:'',arrangeInfo:[{weekDay:4,weekState:'01111111111111111000000000000000000000000000000000000',startUnit:3,endUnit:5,weekStateDigest:'1-16',rooms:'HGX509'}]},{id:698251,no:'ECON130010.01',name:'当代中国经济',teachDepartName:'经济学院',code:'ECON130010',credits:3.0,courseId:38088,examTime:'2022-12-28 08:30-10:30 第17周 星期三',examFormName:'闭卷',startWeek:11,endWeek:16,courseTypeId:7,courseTypeName:'专业必修课程',courseTypeCode:'03_01',scheduled:true,hasTextBook:false,period:54,weekHour:3.0,withdrawable:true,textbooks:'',teachers:'陈钊,王永钦,张晏',campusCode:'H',campusName:'邯郸校区',remark:'国家级一流本科课程',arrangeInfo:[{weekDay:3,weekState:'00000011111000000000000000000000000000000000000000000',startUnit:3,endUnit:5,weekStateDigest:'6-10',rooms:'H4305'},{weekDay:3,weekState:'01111100000000000000000000000000000000000000000000000',startUnit:3,endUnit:5,weekStateDigest:'1-5',rooms:'H4305'},{weekDay:3,weekState:'00000000000111111000000000000000000000000000000000000',startUnit:3,endUnit:5,weekStateDigest:'11-16',rooms:'H4305'}]},{id:698260,no:'ECON130042.01',name:'税收学',teachDepartName:'经济学院',code:'ECON130042',credits:3.0,courseId:38120,examTime:'2022-12-28 13:00-15:00 第17周 星期三',examFormName:'闭卷',startWeek:1,endWeek:18,courseTypeId:7,courseTypeName:'专业必修课程',courseTypeCode:'03_01',scheduled:true,hasTextBook:false,period:54,weekHour:3.0,withdrawable:true,textbooks:'',teachers:'余显财',campusCode:'H',campusName:'邯郸校区',remark:'',arrangeInfo:[{weekDay:5,weekState:'01111111111111111000000000000000000000000000000000000',startUnit:3,endUnit:5,weekStateDigest:'1-16',rooms:'H6108'}]},{id:698266,no:'ECON130064.01',name:'博弈论',teachDepartName:'经济学院',code:'ECON130064',credits:3.0,courseId:38142,examTime:'2023-01-05 13:00-15:00 第18周 星期四',examFormName:'闭卷',startWeek:1,endWeek:16,courseTypeId:7,courseTypeName:'专业必修课程',courseTypeCode:'03_01',scheduled:true,hasTextBook:false,period:54,weekHour:3.0,withdrawable:true,textbooks:'',teachers:'朱弘鑫',campusCode:'H',campusName:'邯郸校区',remark:'',arrangeInfo:[{weekDay:4,weekState:'01111111111111111000000000000000000000000000000000000',startUnit:6,endUnit:8,weekStateDigest:'1-16',rooms:'H6212'}]},{id:698275,no:'ECON130128.01',name:'制度经济学',teachDepartName:'经济学院',code:'ECON130128',credits:3.0,courseId:38206,examTime:'2022-12-30 15:30-17:30 第17周 星期五',examFormName:'开卷',startWeek:1,endWeek:16,courseTypeId:12,courseTypeName:'专业选修课程',courseTypeCode:'03_02',scheduled:true,hasTextBook:false,period:54,weekHour:3.0,withdrawable:true,textbooks:'',teachers:'方钦',campusCode:'H',campusName:'邯郸校区',remark:'',arrangeInfo:[{weekDay:5,weekState:'01111111111111111000000000000000000000000000000000000',startUnit:6,endUnit:8,weekStateDigest:'1-16',rooms:'H6306'}]}]";
        const AMOUNT: &str = "{'698241':{sc:70,lc:100},'698246':{sc:89,lc:100},'698257':{sc:74,lc:85},'698251':{sc:85,lc:85},'698260':{sc:39,lc:40},'698266':{sc:93,lc:93},'698275':{sc:32,lc:32}}";
        let course_str = normalize_json(COURSE);
        let amount_str = normalize_json(AMOUNT);
        println!("{}\n{}", course_str, amount_str);

        let course: Vec<Course> = serde_json::from_str(&course_str).unwrap();
        let amount: HashMap<String, AmountInfo> = serde_json::from_str(&amount_str).unwrap();
        println!("{:?}", course);
        println!("{:?}", amount);
    }
}