use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    List(Vec<Value>),
    Nil,
    Symbol(String),
    Keyword(String),
    Map(HashMap<String, Value>),
}
