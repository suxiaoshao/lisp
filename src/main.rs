use std::collections::HashMap;

use errors::LispError;
use rustyline::{DefaultEditor, error::ReadlineError};

mod errors;
mod parse;
mod process;
mod root;
mod value;

#[cfg(test)]
mod test_utils;

fn main() -> Result<(), LispError> {
    let mut rl = DefaultEditor::new()?;
    let arena = root::GcArena::new(|mc| {
        let mut vars = HashMap::new();
        vars.insert("#f".to_string(), value::Value::Boolean(false));
        vars.insert("#t".to_string(), value::Value::Boolean(true));
        root::LispRoot {
            variables: Gc::new(mc, RefLock::new(vars)),
        }
    });

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;

                let result: Result<String, LispError> = arena.mutate(|mc, root| {
                    let (_, expression) = crate::parse::parse_expression(mc, &line)
                        .map_err(|_| LispError::InvalidInput)?;
                    println!("{expression}");
                    let vars = HashMap::new();
                    let value = expression
                        .eval(root, &vars, mc)
                        .map_err(LispError::ComputerError)?;
                    Ok(format!("{}", value))
                });

                match result {
                    Ok(data) => println!("Result: {}", data),
                    Err(err) => println!("Error:{err}"),
                }

                if line.trim() == "exit" {
                    break;
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {}", err);
                break;
            }
        }
    }
    Ok(())
}
