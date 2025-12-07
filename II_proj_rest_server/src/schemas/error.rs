use serde;

#[derive(serde::Serialize, Clone)]
pub struct MultipleProblemsError
{
    problems: Vec<Problem>
}

impl MultipleProblemsError
{
    pub fn new(problems: Vec<Problem>) -> MultipleProblemsError
    {
        MultipleProblemsError {problems}
    }

    pub fn new_with_one_problem(
        error_msg: &str, 
        ctx: &str
    ) -> MultipleProblemsError
    {
        MultipleProblemsError {
            problems: vec![
                Problem::new(
                    &Error::new(error_msg),
                    &ctx
                )
            ]
        }
    }
}

#[derive(serde::Serialize, Clone)]
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