use crate::{errors::LispComputerError, parse::parse_expression, root::GcArena, value::Value};
use gc_arena::lock::RefLock;
use std::collections::HashMap;

/// Evaluate a string expression in the given arena and return its display string.
pub fn eval_str<'gc>(
    input: &str,
    arena: &'gc mut GcArena<'gc>,
) -> Result<String, LispComputerError> {
    arena.mutate(|mc, root| -> Result<String, LispComputerError> {
        let (_, expr) = parse_expression(mc, input)
            .map_err(|_| LispComputerError::InvalidExpression("parse error".to_string()))?;
        let vars = HashMap::new();
        let value = expr.eval(root, &vars, mc)?;
        Ok(format!("{}", value))
    })
}
