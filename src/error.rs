use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
	#[error("Invalid UTF-8")]
	InvalidUtf8(#[from] std::str::Utf8Error),
	#[error("Invalid integer")]
	ParseInt(#[from] std::num::ParseIntError),
	#[error("Invalid float")]
	ParseFloat(#[from] std::num::ParseFloatError),
	#[error("Error: {0}")]
	MissingValue(String),
	#[error("Invalid playlist type")]
	InvalidPlaylistType,
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Context<T> {
	fn context<S: Into<String>>(self, context: S) -> Result<T>;
}

impl<T> Context<T> for Option<T> {
	fn context<S: Into<String>>(self, context: S) -> Result<T> {
		match self {
			Some(value) => Ok(value),
			None => Err(Error::MissingValue(context.into())),
		}
	}
}
