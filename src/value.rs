use std::collections::HashMap;

mod lamda;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    List(Vec<Value>),
    Nil,
    Lambda(lamda::Lambda),
    Symbol(String),
    Keyword(String),
    Map(HashMap<String, Value>),
}
