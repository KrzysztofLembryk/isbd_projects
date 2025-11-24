use serde;

#[derive(serde::Serialize)]
pub struct MultipleProblemsError
{
    problems: Vec<Problem>
}

#[derive(serde::Serialize)]
pub struct Problem
{
    error: Error,
    context: String
}


impl Problem
{
    pub fn new(error: &Error, ctx: &str) -> Problem
    {
        Problem { error: error.clone(), context: String::from(ctx) }
    }
}

#[derive(serde::Serialize, Clone)]
pub struct Error
{
    message: String
}

impl Error
{
    pub fn new(msg: &str) -> Error
    {
        Error {message: String::from(msg)}
    }
}