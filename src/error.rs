use std::fmt::{Debug, Display, Formatter};

pub type Result<T> = std::result::Result<T, SDKError>;

#[derive(Default)]
pub struct SDKError {
    message: String,
    cause: Option<Box<dyn Display>>,
}

impl SDKError {
    pub fn is_none_error(&self) -> bool {
        self.cause.is_none() && self.message.is_empty()
    }
    pub fn none() -> Self { Default::default() }
    pub fn new(message: String) -> Self {
        SDKError {
            message,
            cause: None,
        }
    }
    pub fn with_cause(message: String, cause: Box<dyn Display>) -> Self {
        SDKError {
            message,
            cause: Some(cause),
        }
    }
}

impl Display for SDKError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.cause {
            Some(cause) => write!(f, "{} caused by {}", self.message, cause),
            None => write!(f, "{}", self.message),
        }
    }
}

impl Debug for SDKError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.cause {
            Some(cause) => write!(f, "{}, cause is {}.", self.message, cause),
            None => write!(f, "{}, no cause.", self.message),
        }
    }
}

impl std::error::Error for SDKError {}

impl From<reqwest::Error> for SDKError {
    fn from(e: reqwest::Error) -> Self {
        SDKError {
            message: "reqwest error".to_string(),
            cause: Some(Box::new(e)),
        }
    }
}

impl From<serde_json::error::Error> for SDKError {
    fn from(e: serde_json::error::Error) -> Self {
        SDKError {
            message: "serde_json error".to_string(),
            cause: Some(Box::new(e)),
        }
    }
}
