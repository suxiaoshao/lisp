use errors::LispError;

mod environment;
mod errors;
mod parse;
mod process;
mod value;

use parse::parse_expression;
use rustyline::{DefaultEditor, error::ReadlineError};

fn main() -> Result<(), LispError> {
    let mut rl = DefaultEditor::new()?;
    let env = environment::Environment::default();

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;

                let (_, expression) =
                    parse_expression(&line).map_err(|_| LispError::InvalidInput)?;

                println!("{expression:?}");

                let result = expression.eval(&env)?;

                println!("Result: {:?}", result);
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
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}
