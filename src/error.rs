use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{}", _0)]
    Io(#[from] std::io::Error),
    #[error("{}", _0)]
    Deserialize(#[from] toml::de::Error),
    #[error("{}", _0)]
    Mustache(#[from] mustache::Error),
    #[error("{}: {}", context, inner)]
    Context {
        context: String,
        inner: Box<Error>,
    }
}

pub trait ErrorExt {
    fn context(self, context: impl Into<String>) -> Error;
}

impl<E: Into<Error>> ErrorExt for E {
    fn context(self, context: impl Into<String>) -> Error {
        Error::Context {
            context: context.into(),
            inner: Box::new(self.into())
        }
    }
}

pub trait ResultExt {
    type Result;

    fn context(self, context: impl Into<String>) -> Self::Result;
    fn with_context(self, func: impl FnOnce() -> String) -> Self::Result;
}

impl<T, E: ErrorExt> ResultExt for Result<T, E> {
    type Result = Result<T, Error>;

    fn context(self, context: impl Into<String>) -> Self::Result {
        self.map_err(|e| e.context(context))
    }

    fn with_context(self, func: impl FnOnce() -> String) -> Self::Result {
        self.map_err(|e| e.context(func()))
    }
}
