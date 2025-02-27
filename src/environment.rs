use std::{cell::RefCell, collections::HashMap};

use crate::{
    errors::LispComputerError,
    parse::Expression,
    process::{
        AdditionProcessor, AndProcessor, CondProcessor, DefineProcessor, DivisionProcessor,
        EqualProcessor, Function, GreaterEqualProcessor, GreaterThanProcessor, IfProcessor,
        LambdaProcessor, LessEqualProcessor, LessThanProcessor, LetProcessor,
        MultiplicationProcessor, OrProcessor, SubtractionProcessor,
    },
    value::Value,
};

pub trait Environment {
    fn process_variable(
        &self,
        symbol: &str,
        args: &[Expression],
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError>;
    fn set_variable(&self, name: String, value: Value);
    fn get_variable(&self, name: &str, variables: &HashMap<&str, Value>) -> Option<Value>;
}

#[derive(Debug)]
pub struct GlobalEnvironment {
    variables: RefCell<HashMap<String, Value>>,
}

impl Default for GlobalEnvironment {
    fn default() -> Self {
        let mut variables = HashMap::new();
        variables.insert("#f".to_string(), Value::Boolean(false));
        variables.insert("#t".to_string(), Value::Boolean(true));
        Self {
            variables: RefCell::new(variables),
        }
    }
}

impl Environment for GlobalEnvironment {
    fn process_variable(
        &self,
        symbol: &str,
        args: &[Expression],
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        if let Some(func) = self.get_language_function(symbol) {
            return func.process(args, self, variables);
        };
        if let Some(Value::Lambda(lamda)) = self.get_variable(symbol, variables) {
            return lamda.process(args, self, variables);
        }
        Err(LispComputerError::UnboundFunction(symbol.to_string()))
    }
    fn set_variable(&self, name: String, value: Value) {
        self.variables.borrow_mut().insert(name, value);
    }
    fn get_variable(&self, name: &str, variables: &HashMap<&str, Value>) -> Option<Value> {
        if let Some(value) = self.variables.borrow().get(name).cloned() {
            Some(value)
        } else {
            variables.get(name).cloned()
        }
    }
}

impl GlobalEnvironment {
    fn get_language_function<T: Environment>(&self, name: &str) -> Option<Box<dyn Function<T>>> {
        let mut functions = Self::language_function_map();
        functions.remove(name)
    }
    fn language_function_map<T: Environment>() -> HashMap<String, Box<dyn Function<T>>> {
        let mut functions: HashMap<String, Box<dyn Function<T>>> = HashMap::new();
        functions.insert(
            <AdditionProcessor as Function<T>>::name(&AdditionProcessor).to_string(),
            Box::new(AdditionProcessor),
        );
        functions.insert(
            <SubtractionProcessor as Function<T>>::name(&SubtractionProcessor).to_string(),
            Box::new(SubtractionProcessor),
        );
        functions.insert(
            <MultiplicationProcessor as Function<T>>::name(&MultiplicationProcessor).to_string(),
            Box::new(MultiplicationProcessor),
        );
        functions.insert(
            <DivisionProcessor as Function<T>>::name(&DivisionProcessor).to_string(),
            Box::new(DivisionProcessor),
        );
        functions.insert(
            <DefineProcessor as Function<T>>::name(&DefineProcessor).to_string(),
            Box::new(DefineProcessor),
        );
        functions.insert(
            <LambdaProcessor as Function<T>>::name(&LambdaProcessor).to_string(),
            Box::new(LambdaProcessor),
        );
        functions.insert(
            <EqualProcessor as Function<T>>::name(&EqualProcessor).to_string(),
            Box::new(EqualProcessor),
        );
        functions.insert(
            <IfProcessor as Function<T>>::name(&IfProcessor).to_string(),
            Box::new(IfProcessor),
        );
        functions.insert(
            <GreaterThanProcessor as Function<T>>::name(&GreaterThanProcessor).to_string(),
            Box::new(GreaterThanProcessor),
        );
        functions.insert(
            <LessThanProcessor as Function<T>>::name(&LessThanProcessor).to_string(),
            Box::new(LessThanProcessor),
        );
        functions.insert(
            <LessEqualProcessor as Function<T>>::name(&LessEqualProcessor).to_string(),
            Box::new(LessEqualProcessor),
        );
        functions.insert(
            <GreaterEqualProcessor as Function<T>>::name(&GreaterEqualProcessor).to_string(),
            Box::new(GreaterEqualProcessor),
        );
        functions.insert(
            <OrProcessor as Function<T>>::name(&OrProcessor).to_string(),
            Box::new(OrProcessor),
        );
        functions.insert(
            <AndProcessor as Function<T>>::name(&AndProcessor).to_string(),
            Box::new(AndProcessor),
        );
        functions.insert(
            <CondProcessor as Function<T>>::name(&CondProcessor).to_string(),
            Box::new(CondProcessor),
        );
        functions.insert(
            <LetProcessor as Function<T>>::name(&LetProcessor).to_string(),
            Box::new(LetProcessor),
        );
        functions
    }
}
