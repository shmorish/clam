#![recursion_limit = "512"]

mod ast;
mod lexer;
mod parser;
mod token;

use lexer::Lexer;
use parser::Parser;
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{Editor, Result};

fn main() -> Result<()> {
    let mut rl: Editor<(), FileHistory> = Editor::new()?;
    let history_file = ".clam_history";

    load_history(&mut rl, history_file);
    run_repl(&mut rl)?;
    save_history(&mut rl, history_file)?;

    Ok(())
}

fn load_history(rl: &mut Editor<(), FileHistory>, history_file: &str) {
    let _ = rl.load_history(history_file);
}

fn save_history(rl: &mut Editor<(), FileHistory>, history_file: &str) -> Result<()> {
    rl.save_history(history_file)?;
    Ok(())
}

fn run_repl(rl: &mut Editor<(), FileHistory>) -> Result<()> {
    loop {
        match rl.readline("$ ") {
            Ok(line) => {
                if !handle_input(rl, &line) {
                    continue;
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
            }
            Err(ReadlineError::Eof) => {
                println!();
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}

fn handle_input(rl: &mut Editor<(), FileHistory>, line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }

    let _ = rl.add_history_entry(line);
    process_command(trimmed);
    true
}

fn process_command(input: &str) {
    let mut lexer = Lexer::new(input);
    match lexer.tokenize() {
        Ok(tokens) => {
            parse_and_display(tokens);
        }
        Err(e) => {
            eprintln!("Lexer error: {}", e);
        }
    }
}

fn parse_and_display(tokens: Vec<token::Token>) {
    let mut parser = Parser::new(tokens);
    match parser.parse() {
        Ok(commands) => {
            match serde_json::to_string_pretty(&commands) {
                Ok(json) => println!("{}", json),
                Err(e) => eprintln!("JSON serialization error: {}", e),
            }
        }
        Err(e) => {
            eprintln!("Parse error: {}", e);
        }
    }
}
