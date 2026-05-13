use cliclack::input;
use crucible::parser::parse_block;
use crucible::runtime::{Environment, execute_block};

fn main() {
    let env = Environment::default();
    let mut last_block = String::new();

    loop {
        let default_input = if last_block.is_empty() {
            "DECLARE ... BEGIN ... END;".to_string()
        } else {
            last_block.clone()
        };

        let input_sql: String = input(
            "Enter PL/SQL block (type EXIT to quit, Shift + Enter for new line, Escape then Enter to submit): ",
        )
        .placeholder("DECLARE ... BEGIN ... END;")
        .default_input(&default_input)
        .multiline()
        .interact()
        .expect("Failed to read input");

        let trimmed = input_sql.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.eq_ignore_ascii_case("EXIT") {
            break;
        }

        last_block = input_sql;

        let block = match parse_block(&trimmed) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Parse error: {}", e);
                continue;
            }
        };

        match execute_block(&block, &env) {
            Ok(value) => println!("{}", value),
            Err(err) => eprintln!("Execution error: {}", err),
        }
    }
}
