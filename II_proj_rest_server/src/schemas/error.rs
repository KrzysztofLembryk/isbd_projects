

pub struct MultipleProblemsError
{
    problems: Vec<Problem>
}

pub struct Problem
{
    error: Error,
    context: String
}

pub struct Error
{
    message: String
}