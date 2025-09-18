#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ParsingError(#[from] ParsingError),
}

#[derive(thiserror::Error, Debug)]
pub enum ParsingError {
    #[error("a command should start with a '/'")]
    NotACommand,
    #[error("malformed command")]
    MalformedCommand,
    #[error("please use a valid command")]
    EmptyCommand,
    #[error("invalid datetime")]
    InvalidDateTime,
}
