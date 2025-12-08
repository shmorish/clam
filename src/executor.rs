use crate::ast::*;
use std::collections::HashMap;
use std::process::{Command as ProcessCommand, Stdio};

pub struct Executor {
    env_vars: HashMap<String, String>,
    last_exit_status: i32,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            env_vars: HashMap::new(),
            last_exit_status: 0,
        }
    }

    pub fn execute(&mut self, command: &crate::ast::Command) -> Result<i32, String> {
        match command {
            Command::Simple(cmd) => self.execute_simple_command(cmd),
            Command::Pipeline(pipeline) => self.execute_pipeline(pipeline),
            Command::List(list) => self.execute_list(list),
            Command::If(if_cmd) => self.execute_if(if_cmd),
            Command::While(while_cmd) => self.execute_while(while_cmd),
            Command::Until(until_cmd) => self.execute_until(until_cmd),
            Command::For(for_cmd) => self.execute_for(for_cmd),
            Command::Redirected(redirected) => self.execute_redirected(redirected),
            _ => Err(format!("Command type not yet implemented: {:?}", command)),
        }
    }

    fn execute_simple_command(&mut self, cmd: &SimpleCommand) -> Result<i32, String> {
        if cmd.words.is_empty() {
            // Assignment-only command
            for assignment in &cmd.assignments {
                self.env_vars.insert(assignment.name.clone(), assignment.value.clone());
            }
            return Ok(0);
        }

        // Expand variables in words and perform word splitting
        let mut expanded_words: Vec<String> = Vec::new();
        for word in &cmd.words {
            let expanded = self.expand_variables(&word.value);
            // Perform word splitting on expanded value
            for split_word in self.word_split(&expanded) {
                expanded_words.push(split_word);
            }
        }

        if expanded_words.is_empty() {
            return Ok(0);
        }

        let program = &expanded_words[0];
        let args: Vec<&str> = expanded_words[1..].iter().map(|s| s.as_str()).collect();

        let mut process = ProcessCommand::new(program);
        process.args(&args);

        // Apply assignments as environment variables
        for assignment in &cmd.assignments {
            process.env(&assignment.name, &assignment.value);
        }

        // Add existing environment variables
        for (key, value) in &self.env_vars {
            process.env(key, value);
        }

        match process.status() {
            Ok(status) => {
                let exit_code = status.code().unwrap_or(1);
                self.last_exit_status = exit_code;
                Ok(exit_code)
            }
            Err(e) => Err(format!("Failed to execute '{}': {}", program, e)),
        }
    }

    fn execute_pipeline(&mut self, _pipeline: &Pipeline) -> Result<i32, String> {
        Err("Pipeline execution not yet implemented".to_string())
    }

    fn execute_list(&mut self, list: &List) -> Result<i32, String> {
        let mut last_status = 0;

        for item in &list.items {
            last_status = self.execute(&item.command)?;

            match item.separator {
                Separator::And => {
                    // && - execute next only if this succeeded
                    if last_status != 0 {
                        break;
                    }
                }
                Separator::Or => {
                    // || - execute next only if this failed
                    if last_status == 0 {
                        break;
                    }
                }
                Separator::Sequential | Separator::Background => {
                    // ; or & - always continue
                    // TODO: background jobs
                }
                Separator::Pipe => {
                    // Should not appear in List, only in Pipeline
                }
            }
        }

        Ok(last_status)
    }

    fn execute_if(&mut self, if_cmd: &IfCommand) -> Result<i32, String> {
        let condition_status = self.execute(&if_cmd.condition)?;

        if condition_status == 0 {
            self.execute(&if_cmd.then_part)
        } else {
            // Check elif clauses
            for (elif_condition, elif_body) in &if_cmd.elif_parts {
                let elif_status = self.execute(elif_condition)?;
                if elif_status == 0 {
                    return self.execute(elif_body);
                }
            }

            // Execute else part if present
            if let Some(else_part) = &if_cmd.else_part {
                self.execute(else_part)
            } else {
                Ok(0)
            }
        }
    }

    fn execute_while(&mut self, while_cmd: &WhileCommand) -> Result<i32, String> {
        loop {
            let condition_status = self.execute(&while_cmd.condition)?;
            if condition_status != 0 {
                break;
            }
            self.execute(&while_cmd.body)?;
        }
        Ok(0)
    }

    fn execute_until(&mut self, until_cmd: &UntilCommand) -> Result<i32, String> {
        loop {
            let condition_status = self.execute(&until_cmd.condition)?;
            if condition_status == 0 {
                break;
            }
            self.execute(&until_cmd.body)?;
        }
        Ok(0)
    }

    fn execute_for(&mut self, for_cmd: &ForCommand) -> Result<i32, String> {
        for word in &for_cmd.words {
            self.env_vars.insert(for_cmd.variable.clone(), word.clone());
            self.execute(&for_cmd.body)?;
        }
        Ok(0)
    }

    fn execute_redirected(&mut self, _redirected: &RedirectedCommand) -> Result<i32, String> {
        Err("Redirected command execution not yet implemented".to_string())
    }

    pub fn get_last_exit_status(&self) -> i32 {
        self.last_exit_status
    }

    fn expand_variables(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$' {
                if chars.peek() == Some(&'{') {
                    // ${VAR} syntax
                    chars.next(); // consume '{'
                    let mut var_name = String::new();

                    while let Some(&c) = chars.peek() {
                        if c == '}' {
                            chars.next(); // consume '}'
                            break;
                        }
                        var_name.push(chars.next().unwrap());
                    }

                    result.push_str(&self.get_variable(&var_name));
                } else {
                    // $VAR syntax
                    let mut var_name = String::new();

                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            var_name.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }

                    result.push_str(&self.get_variable(&var_name));
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    fn get_variable(&self, name: &str) -> String {
        // Check shell variables first
        if let Some(value) = self.env_vars.get(name) {
            return value.clone();
        }

        // Then check environment variables
        std::env::var(name).unwrap_or_default()
    }

    fn word_split(&self, input: &str) -> Vec<String> {
        // Split on whitespace (spaces, tabs, newlines)
        // This is a simplified version - real bash uses IFS variable
        input
            .split_whitespace()
            .map(|s| s.to_string())
            .collect()
    }
}
