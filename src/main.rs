use errors::LispError;

mod errors;
mod parse;
mod process;

use parse::parse_expression;
use process::process_expression;
use rustyline::{error::ReadlineError, DefaultEditor};

fn main() -> Result<(), LispError> {
    // 创建一个 rustyline 编辑器
    let mut rl = DefaultEditor::new()?;

    loop {
        // 读取用户输入
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;

                let (_, expression) =
                    parse_expression(&line).map_err(|_| LispError::InvalidInput)?;

                println!("{expression:?}");

                let result = process_expression(&expression)?;

                println!("Result: {}", result);

                // 自定义退出条件
                if line.trim() == "exit" {
                    break;
                }
            }
            Err(ReadlineError::Interrupted) => {
                // 用户使用 Ctrl-C 中断
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                // 用户使用 Ctrl-D 退出
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                // 其他错误
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}
