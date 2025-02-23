use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
	#[error("Generic Error: {0}")]
	Generic(String),
}

impl From<String> for Error {
	fn from(s: String) -> Self {
		Error::Generic(s)
	}
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Context<T> {
	fn context<S: Into<String>>(self, context: S) -> Result<T>;
}

impl<T> Context<T> for Option<T> {
	fn context<S: Into<String>>(self, context: S) -> Result<T> {
		match self {
			Some(value) => Ok(value),
			None => Err(Error::Generic(context.into())),
		}
	}
}

#[macro_export]
macro_rules! bail {
	($($arg:tt)*) => {
		return Err(Error::Generic(format!($($arg)*)))
	};
}
