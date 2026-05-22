use std::io::{self, Write};

use cliclack::input;

use crate::parser::parse_block;
use crate::runtime::{Environment, execute_block_collect_values};

const DEFAULT_PROMPT: &str = "DECLARE ... BEGIN ... END;";

pub fn run() {
    let env = Environment::default();
    let mut last_block = String::new();

    clear_screen();

    loop {
        let input_sql = read_block(&last_block);
        let trimmed = input_sql.trim().to_string();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed.eq_ignore_ascii_case("EXIT") {
            break;
        }

        last_block = input_sql;

        let block = match parse_block(&trimmed) {
            Ok(block) => block,
            Err(err) => {
                eprintln!("Parse error: {}", err);
                continue;
            }
        };

        match execute_block_collect_values(&block, &env) {
            Ok(values) => {
                for value in values {
                    println!("{}", value);
                }
            }
            Err(err) => eprintln!("Execution error: {}", err),
        }
    }
}

fn read_block(last_block: &str) -> String {
    let default_input = if last_block.is_empty() {
        DEFAULT_PROMPT.to_string()
    } else {
        last_block.to_string()
    };

    input(
        "Enter PL/SQL block (type EXIT to quit, Shift + Enter for new line, Escape then Enter to submit): ",
    )
    .placeholder(DEFAULT_PROMPT)
    .default_input(&default_input)
    .multiline()
    .interact()
    .expect("Failed to read input")
}

fn clear_screen() {
    print!("\x1B[2J\x1B[H");
    io::stdout().flush().expect("Failed to clear screen");
}
