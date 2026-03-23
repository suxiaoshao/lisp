use gc_arena::lock::RefLock;
use gc_arena::{Arena, Gc, Mutation, Rootable};
use gc_arena_derive::Collect;
use std::collections::HashMap;

use crate::{
    environment::Environment,
    errors::LispComputerError,
    parse::Expression,
    process::{
        AdditionProcessor, AndProcessor, CondProcessor, DefineProcessor, DivisionProcessor,
        DoProcessor, EqualProcessor, Function, GreaterEqualProcessor, GreaterThanProcessor,
        IfProcessor, LambdaProcessor, LessEqualProcessor, LessThanProcessor, LetProcessor,
        MultiplicationProcessor, OrProcessor, SubtractionProcessor,
    },
    value::Value,
};

/// Root struct for the GC arena, holding global variables.
#[derive(Collect)]
#[collect(no_drop)]
pub struct LispRoot<'gc> {
    pub variables: Gc<'gc, RefLock<HashMap<String, Value<'gc>>>>,
}

/// A token type that implements Rootable for any lifetime, linking to LispRoot.
pub struct RootToken;
impl<'gc> Rootable<'gc> for RootToken {
    type Root = LispRoot<'gc>;
}

/// Type alias for the GC arena.
pub type GcArena<'gc> = Arena<RootToken>;

impl<'gc> Environment<'gc> for LispRoot<'gc> {
    fn process_variable(
        &self,
        symbol: &str,
        args: &[Gc<'gc, Expression<'gc>>],
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        if let Some(func) = self.get_language_function(symbol) {
            return func.process(args, self, variables, mc);
        };
        if let Some(Value::Lambda(lamda)) = self.get_variable(symbol, variables) {
            return lamda.process(args, self, variables, mc);
        }
        Err(LispComputerError::UnboundFunction(symbol.to_string()))
    }

    fn set_variable(&self, name: String, value: Value<'gc>, mc: &'gc Mutation<'gc>) {
        self.variables.borrow_mut(mc).insert(name, value);
    }

    fn get_variable(
        &self,
        name: &str,
        variables: &HashMap<String, Value<'gc>>,
    ) -> Option<Value<'gc>> {
        if let Some(value) = self.variables.borrow().get(name).cloned() {
            Some(value)
        } else {
            variables.get(name).cloned()
        }
    }

    fn is_builtin(&self, name: &str) -> bool {
        self.get_language_function::<Self>(name).is_some()
    }
}

impl<'gc> LispRoot<'gc> {
    fn get_language_function<T: Environment<'gc>>(
        &self,
        name: &str,
    ) -> Option<Box<dyn Function<'gc, T>>> {
        let mut functions = Self::language_function_map();
        functions.remove(name)
    }

    fn language_function_map<T: Environment<'gc>>() -> HashMap<String, Box<dyn Function<'gc, T>>> {
        let mut functions: HashMap<String, Box<dyn Function<'gc, T>>> = HashMap::new();
        functions.insert(
            <AdditionProcessor as Function<'gc, T>>::name(&AdditionProcessor).to_string(),
            Box::new(AdditionProcessor),
        );
        functions.insert(
            <SubtractionProcessor as Function<'gc, T>>::name(&SubtractionProcessor).to_string(),
            Box::new(SubtractionProcessor),
        );
        functions.insert(
            <MultiplicationProcessor as Function<'gc, T>>::name(&MultiplicationProcessor)
                .to_string(),
            Box::new(MultiplicationProcessor),
        );
        functions.insert(
            <DivisionProcessor as Function<'gc, T>>::name(&DivisionProcessor).to_string(),
            Box::new(DivisionProcessor),
        );
        functions.insert(
            <DefineProcessor as Function<'gc, T>>::name(&DefineProcessor).to_string(),
            Box::new(DefineProcessor),
        );
        functions.insert(
            <LambdaProcessor as Function<'gc, T>>::name(&LambdaProcessor).to_string(),
            Box::new(LambdaProcessor),
        );
        functions.insert(
            <EqualProcessor as Function<'gc, T>>::name(&EqualProcessor).to_string(),
            Box::new(EqualProcessor),
        );
        functions.insert(
            <IfProcessor as Function<'gc, T>>::name(&IfProcessor).to_string(),
            Box::new(IfProcessor),
        );
        functions.insert(
            <GreaterThanProcessor as Function<'gc, T>>::name(&GreaterThanProcessor).to_string(),
            Box::new(GreaterThanProcessor),
        );
        functions.insert(
            <LessThanProcessor as Function<'gc, T>>::name(&LessThanProcessor).to_string(),
            Box::new(LessThanProcessor),
        );
        functions.insert(
            <LessEqualProcessor as Function<'gc, T>>::name(&LessEqualProcessor).to_string(),
            Box::new(LessEqualProcessor),
        );
        functions.insert(
            <GreaterEqualProcessor as Function<'gc, T>>::name(&GreaterEqualProcessor).to_string(),
            Box::new(GreaterEqualProcessor),
        );
        functions.insert(
            <OrProcessor as Function<'gc, T>>::name(&OrProcessor).to_string(),
            Box::new(OrProcessor),
        );
        functions.insert(
            <AndProcessor as Function<'gc, T>>::name(&AndProcessor).to_string(),
            Box::new(AndProcessor),
        );
        functions.insert(
            <CondProcessor as Function<'gc, T>>::name(&CondProcessor).to_string(),
            Box::new(CondProcessor),
        );
        functions.insert(
            <LetProcessor as Function<'gc, T>>::name(&LetProcessor).to_string(),
            Box::new(LetProcessor),
        );
        functions.insert(
            <DoProcessor as Function<'gc, T>>::name(&DoProcessor).to_string(),
            Box::new(DoProcessor),
        );
        functions
    }
}
