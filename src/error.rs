use std::error;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    LoginError,
    LogoutError,
    Parse(reqwest::Error), // wrap http errors
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::LoginError => write!(f, "login error"),
            Error::LogoutError => write!(f, "logout error"),
            Error::Parse(..) => write!(f, "error"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::Parse(ref e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Error {
        Error::Parse(err)
    }
}