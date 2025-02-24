use crate::parse::Expression;

#[derive(Debug, Clone, PartialEq)]
pub struct Lambda {
    name: String,
    params: Vec<String>,
    body: Vec<Expression>,
}
