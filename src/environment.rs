use std::collections::HashMap;

use crate::process::{
    AdditionProcessor, DivisionProcessor, Function, MultiplicationProcessor, SubtractionProcessor,
};

pub struct Environment {
    functions: HashMap<String, Box<dyn Function>>,
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

        Self { functions }
    }
}

impl Environment {
    pub fn get_function(&self, name: &str) -> Option<&Box<dyn Function>> {
        self.functions.get(name)
    }
}
