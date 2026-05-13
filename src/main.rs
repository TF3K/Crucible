use cliclack::input;
use crucible::parser::parse_block;
use crucible::runtime::{Environment, execute_block};

fn main() {
    let input_sql: String = input("Enter PL/SQL block (Shift + Enter for new line, Escape then Enter to submit): ")
        .placeholder("DECLARE ... BEGIN ... END;")
        .default_input("")
        .multiline()
        .interact()
        .expect("Failed to read input");

    let trimmed = input_sql.trim();
    if trimmed.is_empty() {
        eprintln!("No input provided");
        return;
    }

    let block = match parse_block(trimmed) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            return;
        }
    };

    let env = Environment::default();

    match execute_block(&block, &env) {
        Ok(value) => println!("{}", value),
        Err(err) => eprintln!("Execution error: {}", err),
    }
}
