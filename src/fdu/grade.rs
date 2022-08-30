use std::fmt::{Display, Formatter};
use std::sync::Arc;

use reqwest::blocking::Client;
use reqwest::cookie::Jar;
use scraper::{Html, Selector};

use super::prelude::*;

struct Grade {
    fdu: Fdu,
    grades: Vec<CourseGrade>,
}

impl Grade {
    fn new() -> Self {
        Self {
            fdu: Fdu::new(),
            grades: Vec::new(),
        }
    }

    fn new_from_fdu(fdu: Fdu) -> Self {
        Self {
            fdu,
            grades: Vec::new(),
        }
    }
}

impl HttpClient for Grade {
    fn get_client(&self) -> &Client {
        self.fdu.get_client()
    }

    fn get_cookie_store(&self) -> &Arc<Jar> {
        self.fdu.get_cookie_store()
    }
}

#[derive(Clone, Debug)]
struct CourseGrade {
    code: String,
    name: String,
    year: String,
    semester: String,
    credit: f64,
    grade: String,
    point: f64,
}

#[derive(Default)]
struct GPA {
    gpa: f64,
    ranking: i32,
    total: i32,
    percentage: f64,
    credits: f64,
}

impl Display for GPA {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f, "gpa: {}, ranking: {}/{} {:.1}%, credits: {}",
            self.gpa, self.ranking, self.total, self.percentage * 100.0, self.credits
        )
    }
}

impl Grade {
    fn get_all_grades(&mut self) -> Result<Vec<CourseGrade>> {
        if self.grades.len() != 0 {
            return Ok(self.grades.to_vec());
        }

        const GRADE_URL: &str = "https://my.fudan.edu.cn/list/bks_xx_cj";
        let mut grades: Vec<CourseGrade> = Vec::new();

        let html = self.send_and_get_text(self.get_client().get(GRADE_URL))?;
        let document = Html::parse_document(html.as_str());
        for tr in document.select(&Selector::parse("tbody tr").unwrap()) {
            let v = tr.text().collect::<Vec<_>>();
            grades.push(CourseGrade {
                code: v[0].to_string(),
                year: v[1].to_string(),
                semester: v[2].to_string(),
                name: v[3].to_string(),
                credit: v[4].parse::<f64>().expect("parse credict error"),
                grade: v[5].to_string(),
                point: grade_to_point(v[5]),
            });
        }

        self.grades = grades;
        Ok(self.grades.to_vec())
    }

    fn get_grades_of_this_semester(&mut self) -> Result<Vec<CourseGrade>> {
        if self.get_all_grades()?.len() == 0 {
            return Ok(Vec::new());
        }
        let year = &self.grades[0].year;
        let semester = &self.grades[0].semester;
        let mut i = 0;
        for grade in &self.grades[..] {
            if !grade.year.eq(year) || !grade.semester.eq(semester) {
                break;
            }
            i += 1;
        }
        Ok(self.grades[..i].to_vec())
    }

    fn get_gpa(&mut self) -> GPA {
        let result = self.get_gpa_from_jwfw();
        if let Ok(gpa) = result {
            return gpa;
        }
        println!("get gpa from jwfw failed, calculate manually");

        let result = self.get_gpa_from_grades();
        if let Ok(gpa) = result {
            return gpa;
        }
        println!("get gpa from grades failed");
        GPA::default()
    }

    fn get_gpa_from_grades(&mut self) -> Result<GPA> {
        let grades = self.get_all_grades()?;
        if grades.len() == 0 {
            return Ok(GPA::default());
        }
        let mut gpa = GPA::default();
        for grade in grades {
            if grade.grade.eq("P") { // P isn't calculated
                continue;
            }
            gpa.gpa += grade.point * grade.credit;
            gpa.credits += grade.credit;
        }
        gpa.gpa /= gpa.credits;
        Ok(gpa)
    }

    fn get_gpa_from_jwfw(&mut self) -> Result<GPA> {
        let mut gpa = GPA::default();
        let mut major = "";

        // get data
        const GPA_SEARCH_URL: &str = "https://jwfw.fudan.edu.cn/eams/myActualGpa!search.action";
        let html = self.send_and_get_text(
            self.get_client().get(GPA_SEARCH_URL)
        )?;
        let document = Html::parse_document(html.as_str());
        let selector = Selector::parse("tbody tr").unwrap();

        // it contains all majors in a school, so we have to find my major
        for tr in document.select(&selector) {
            let mut v = tr.text().collect::<Vec<_>>();
            v.retain(|&x| x.trim() != "");
            if !v[0].starts_with("*") { // it's me!
                major = v[3];
                gpa.gpa = v[5].parse::<f64>().expect("parse gpa error");
                gpa.credits = v[6].parse::<f64>().expect("parse credits error");
                break;
            }
        }

        // find ranking, because records are in descending order
        for tr in document.select(&selector) {
            let mut v = tr.text().collect::<Vec<_>>();
            v.retain(|&x| x.trim() != "");
            if v[3] != major {
                continue;
            }
            // my major
            gpa.total += 1;
            if !v[0].starts_with("*") { // it's me!
                gpa.ranking = gpa.total
            }
        }

        if gpa.total != 0 { // calculate percentage
            gpa.percentage = gpa.ranking as f64 / gpa.total as f64;
        }

        Ok(gpa)
    }
}

fn grade_to_point(grade: &str) -> f64 {
    match grade {
        "A" => 4.0,
        "A-" => 3.7,
        "B+" => 3.3,
        "B" => 3.0,
        "B-" => 2.7,
        "C+" => 2.3,
        "C" => 2.0,
        "C-" => 1.7,
        "D+" => 1.3,
        "D" => 1.0,
        "F" => 0.0,
        "P" => 0.0,
        _ => {
            println!("[W] unknown grade {}", grade);
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::fdu::fdu::Account;

    use super::*;

    #[test]
    fn test_get_grades() {
        dotenv::dotenv().ok();  // load env from .env file
        let uid = std::env::var("UID").expect("environment variable UID not set");
        let pwd = std::env::var("PWD").expect("environment variable PWD not set");

        let mut grade = Grade::new();
        grade.fdu.login(uid.as_str(), pwd.as_str()).unwrap();

        grade.get_all_grades().expect("get all grades fail");
        let grades = grade.get_grades_of_this_semester().expect("get grades of this semester fail");
        println!("{:#?}", grades);

        grade.fdu.logout().unwrap();
    }

    #[test]
    fn test_get_gpa() {
        dotenv::dotenv().ok();  // load env from .env file
        let uid = std::env::var("UID").expect("environment variable UID not set");
        let pwd = std::env::var("PWD").expect("environment variable PWD not set");

        let mut grade = Grade::new();
        grade.fdu.login(uid.as_str(), pwd.as_str()).unwrap();

        let gpa = grade.get_gpa_from_jwfw().expect("get gpa fail");
        assert_ne!(gpa.gpa, 0.0);

        let gpa = grade.get_gpa_from_grades().expect("get gpa fail");
        assert_ne!(gpa.gpa, 0.0);

        let gpa = grade.get_gpa();
        assert_ne!(gpa.gpa, 0.0);

        grade.fdu.logout().unwrap();
    }
}