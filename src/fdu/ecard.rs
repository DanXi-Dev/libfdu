use scraper::{Html, Selector};
use crate::fdu::fdu::{Account, Fdu};

impl ECardClient for Fdu {}

const ECARD_QR_CODE_URL: &str = "https://ecard.fudan.edu.cn/epay/wxpage/fudan/zfm/qrcode";

pub trait ECardClient: Account {
    fn get_qr_code(&self) -> reqwest::Result<String> {
        let client = self.get_client();
        let mut html = client.get(ECARD_QR_CODE_URL).send()?.text()?;
        let document = Html::parse_document(html.as_str());
        let selector = Selector::parse(r##"#myText"##).unwrap();
        let element = document.select(&selector).next().unwrap();
        Ok(element.value().attr("value").unwrap().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_code() {
        dotenv::dotenv().ok();  // load env from .env file
        let uid = std::env::var("UID").expect("environment variable UID not set");
        let pwd = std::env::var("PWD").expect("environment variable PWD not set");

        let mut fd = Fdu::new();
        fd.login(uid.as_str(), pwd.as_str()).expect("login error");
        println!("{:#?}", fd.get_qr_code());
        fd.logout().expect("logout error");
    }
}
