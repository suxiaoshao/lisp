//! Lisp REPL (Read-Eval-Print Loop) entry point.
//!
//! This module provides an interactive REPL for the Lisp interpreter using
//! the `rustyline` library for line editing and history support.

use lisp::errors::LispError;
use lisp::root::GcArena;
use rustyline::{DefaultEditor, error::ReadlineError};
use std::collections::HashMap;

/// Main entry point for the Lisp REPL.
///
/// # Features
/// - Read expressions from stdin with line editing support
/// - Parse expressions using the Lisp parser
/// - Evaluate in a fresh GC arena with global environment
/// - Print results or errors
/// - Maintain command history
///
/// # Controls
/// - Type `exit` to quit
/// - Ctrl-C to interrupt current input
/// - Ctrl-D to exit (EOF)
///
/// # Errors
/// Returns `LispError` if the rustyline editor cannot be initialized.
fn main() -> Result<(), LispError> {
    let mut rl = DefaultEditor::new()?;
    let arena = GcArena::new(|mc| lisp::root::LispRoot::new(mc));

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;

                let result: Result<String, LispError> = arena.mutate(|mc, root| {
                    let (_, expression) = lisp::parse::parse_expression(mc, &line)
                        .map_err(|_| LispError::InvalidInput)?;
                    println!("{expression}");
                    let vars = HashMap::new();
                    let value = expression.eval(root, &vars, mc)?;
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
