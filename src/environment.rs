use std::{cell::RefCell, collections::HashMap};

use crate::{
    process::{
        AdditionProcessor, DefineProcessor, DivisionProcessor, Function, MultiplicationProcessor,
        SubtractionProcessor,
    },
    value::Value,
};

pub struct Environment {
    functions: HashMap<String, Box<dyn Function>>,
    variables: RefCell<HashMap<String, Value>>,
}

impl Default for Environment {
    fn default() -> Self {
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

        Self {
            functions,
            variables: RefCell::new(HashMap::new()),
        }
    }
}

impl Environment {
    pub fn get_function(&self, name: &str) -> Option<&Box<dyn Function>> {
        self.functions.get(name)
    }
    pub fn set_variable(&self, name: String, value: Value) {
        self.variables.borrow_mut().insert(name, value);
    }
    pub fn get_variable(&self, name: &str) -> Option<Value> {
        self.variables.borrow().get(name).cloned()
    }
}
