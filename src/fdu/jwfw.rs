use scraper::{Html, Selector};
use crate::fdu::fdu::{Account, Fdu};

const JWFW_URL: &str = "https://jwfw.fudan.edu.cn/eams/home.action";

impl JwfwClient for Fdu {}

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
}