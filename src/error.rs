use std::fmt::{Debug, Display, Formatter};
use serde::de::Unexpected::Str;

pub type Result<T> = std::result::Result<T, SDKError>;

pub enum ErrorType {
    LoginError,
    ParseError,
    NoneError,
    OtherError,
}

impl Display for ErrorType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorType::LoginError => write!(f, "LoginError"),
            ErrorType::ParseError => write!(f, "ParseError"),
            ErrorType::NoneError => write!(f, "NoneError"),
            ErrorType::OtherError => write!(f, "OtherError"),
        }
    }
}

#[derive(Default)]
pub struct SDKError {
    r#type: ErrorType,
    message: String,
    cause: Option<Box<dyn Display>>,
}

impl SDKError {
    pub fn is_none_error(&self) -> bool { self.r#type == ErrorType::NoneError }
    pub fn none() -> Self { SDKError::with_type(ErrorType::NoneError, Default::default()) }
    pub fn new(message: String) -> Self {
        SDKError::with_type(ErrorType::NoneError, message)
    }
    pub fn with_type(r#type: ErrorType, message: String) -> Self {
        SDKError {
            r#type,
            message,
            cause: None,
        }
    }
    pub fn with_cause(r#type: ErrorType, message: String, cause: Box<dyn Display>) -> Self {
        SDKError {
            r#type,
            message,
            cause: Some(cause),
        }
    }
}

impl Display for SDKError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.cause {
            Some(cause) => write!(f, "Type {}: {} caused by {}", self.r#type, self.message, cause),
            None => write!(f, "{}", self.message),
        }
    }
}

impl Debug for SDKError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { Display::fmt(self, f) }
}

impl std::error::Error for SDKError {}

impl From<reqwest::Error> for SDKError {
    fn from(e: reqwest::Error) -> Self {
        SDKError::with_cause(ErrorType::OtherError, "reqwest reported an error".to_string(), Box::new(e))
    }
}

impl From<serde_json::error::Error> for SDKError {
    fn from(e: serde_json::error::Error) -> Self {
        SDKError::with_cause(ErrorType::ParseError, "serde_json reported an error".to_string(), Box::new(e))
    }
}