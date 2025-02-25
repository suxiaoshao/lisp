use std::{cell::RefCell, collections::HashMap};

use crate::{
    errors::LispComputerError,
    parse::Expression,
    process::{
        AdditionProcessor, DefineProcessor, DivisionProcessor, EqualProcessor, Function,
        GreaterThanOrEqualProcessor, IfProcessor, LambdaProcessor, LessThanOrEqualProcessor,
        MultiplicationProcessor, SubtractionProcessor,
    },
    value::Value,
};

#[derive(Debug, Clone)]
pub struct Environment {
    parent: Option<Box<Environment>>,
    variables: RefCell<HashMap<String, Value>>,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            variables: RefCell::new(HashMap::new()),
            parent: None,
        }
    }
}

impl Environment {
    pub fn process_variable(
        &self,
        symbol: &str,
        args: &[Expression],
    ) -> Result<Value, LispComputerError> {
        if let Some(func) = self.get_language_function(symbol) {
            return func.process(args, self);
        };
        if let Some(Value::Lambda(lamda)) = self.get_variable(symbol) {
            return lamda.process(args, self);
        }
        Err(LispComputerError::UnboundFunction(symbol.to_string()))
    }
    pub fn set_variable(&self, name: String, value: Value) {
        self.variables.borrow_mut().insert(name, value);
    }
    pub fn get_variable(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.variables.borrow().get(name).cloned() {
            Some(value)
        } else if let Some(ref parent) = self.parent {
            parent.get_variable(name)
        } else {
            None
        }
    }
    pub fn new_child(&self, variables: HashMap<String, Value>) -> Self {
        Self {
            variables: RefCell::new(variables),
            parent: Some(Box::new(self.clone())),
        }
    }
    fn get_language_function(&self, name: &str) -> Option<Box<dyn Function>> {
        let mut functions = Self::language_function_map();
        functions.remove(name)
    }
    fn language_function_map() -> HashMap<String, Box<dyn Function>> {
        let mut functions: HashMap<String, Box<dyn Function>> = HashMap::new();
        functions.insert(
            AdditionProcessor.name().to_string(),
            Box::new(AdditionProcessor),
        );
        functions.insert(
            SubtractionProcessor.name().to_string(),
            Box::new(SubtractionProcessor),
        );
        functions.insert(
            MultiplicationProcessor.name().to_string(),
            Box::new(MultiplicationProcessor),
        );
        functions.insert(
            DivisionProcessor.name().to_string(),
            Box::new(DivisionProcessor),
        );
        functions.insert(
            DefineProcessor.name().to_string(),
            Box::new(DefineProcessor),
        );
        functions.insert(
            LambdaProcessor.name().to_string(),
            Box::new(LambdaProcessor),
        );
        functions.insert(EqualProcessor.name().to_string(), Box::new(EqualProcessor));
        functions.insert(IfProcessor.name().to_string(), Box::new(IfProcessor));
        functions.insert(
            GreaterThanOrEqualProcessor.name().to_string(),
            Box::new(GreaterThanOrEqualProcessor),
        );
        functions.insert(
            LessThanOrEqualProcessor.name().to_string(),
            Box::new(LessThanOrEqualProcessor),
        );
        functions.insert(
            LessThanOrEqualProcessor.name().to_string(),
            Box::new(LessThanOrEqualProcessor),
        );
        functions.insert(
            GreaterThanOrEqualProcessor.name().to_string(),
            Box::new(GreaterThanOrEqualProcessor),
        );
        functions
    }
}
