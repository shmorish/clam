#![recursion_limit = "512"]

mod ast;
mod executor;
mod lexer;
mod parser;
mod token;

use executor::Executor;
use lexer::Lexer;
use parser::Parser;
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{Editor, Result};

fn main() -> Result<()> {
    let mut rl: Editor<(), FileHistory> = Editor::new()?;
    let mut executor = Executor::new();
    let history_file = ".clam_history";

    load_history(&mut rl, history_file);
    run_repl(&mut rl, &mut executor)?;
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

fn run_repl(rl: &mut Editor<(), FileHistory>, executor: &mut Executor) -> Result<()> {
    loop {
        match rl.readline("$ ") {
            Ok(line) => {
                if !handle_input(rl, executor, &line) {
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

fn handle_input(rl: &mut Editor<(), FileHistory>, executor: &mut Executor, line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }

    let _ = rl.add_history_entry(line);
    process_command(executor, trimmed);
    true
}

fn process_command(executor: &mut Executor, input: &str) {
    let mut lexer = Lexer::new(input);
    match lexer.tokenize() {
        Ok(tokens) => {
            parse_and_execute(executor, tokens);
        }
        Err(e) => {
            eprintln!("Lexer error: {}", e);
        }
    }
}

fn parse_and_execute(executor: &mut Executor, tokens: Vec<token::Token>) {
    let mut parser = Parser::new(tokens);
    match parser.parse() {
        Ok(commands) => {
            for command in commands {
                match executor.execute(&command) {
                    Ok(_exit_status) => {
                        // Command executed successfully
                    }
                    Err(e) => {
                        eprintln!("Execution error: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Parse error: {}", e);
        }
    }
}
