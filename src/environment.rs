use std::collections::HashMap;

use crate::{errors::LispComputerError, parse::Expression, value::Value};
use gc_arena::Mutation;

pub trait Environment<'gc> {
    fn process_variable(
        &self,
        symbol: &str,
        args: &[gc_arena::Gc<'gc, Expression<'gc>>],
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError>;
    fn set_variable(&self, name: String, value: Value<'gc>, mc: &'gc Mutation<'gc>);
    fn get_variable(
        &self,
        name: &str,
        variables: &HashMap<String, Value<'gc>>,
    ) -> Option<Value<'gc>>;
    fn is_builtin(&self, _name: &str) -> bool {
        false
    }
}
