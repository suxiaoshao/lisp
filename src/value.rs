mod lambda;

pub use lambda::Lambda;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
    Boolean(bool),
    Nil,
    Lambda(lambda::Lambda),
}

impl Value {
    pub fn boolean(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            Value::Nil => false,
            _ => true,
        }
    }
}
