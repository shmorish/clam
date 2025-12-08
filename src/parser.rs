use crate::ast::*;
use crate::token::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Command>, String> {
        let mut commands = Vec::new();

        self.skip_newlines();

        while !self.is_at_end() {
            commands.push(self.parse_list()?);
            self.skip_newlines();

            // Break if we've reached EOF or can't make progress
            if self.is_at_end() {
                break;
            }
        }

        Ok(commands)
    }

    // <LIST> ::= <NEWLINE-LIST> <LIST0>
    fn parse_list(&mut self) -> Result<Command, String> {
        self.skip_newlines();
        self.parse_list0()
    }

    // <LIST0> ::= <LIST1> '\n' <NEWLINE-LIST>
    //          | <LIST1> '&' <NEWLINE-LIST>
    //          | <LIST1> ';' <NEWLINE-LIST>
    fn parse_list0(&mut self) -> Result<Command, String> {
        let mut items = Vec::new();
        let first = self.parse_list1()?;

        let separator = if self.check(&TokenKind::Newline) {
            self.advance();
            self.skip_newlines();
            Separator::Sequential
        } else if self.check(&TokenKind::Ampersand) {
            self.advance();
            self.skip_newlines();
            Separator::Background
        } else if self.check(&TokenKind::Semicolon) {
            self.advance();
            self.skip_newlines();
            Separator::Sequential
        } else {
            Separator::Sequential
        };

        items.push(ListItem {
            command: first,
            separator,
        });

        if items.len() == 1 {
            return Ok(items.into_iter().next().unwrap().command);
        }

        Ok(Command::List(List { items }))
    }

    // <LIST1> ::= <LIST1> '&&' <NEWLINE-LIST> <LIST1>
    //          | <LIST1> '||' <NEWLINE-LIST> <LIST1>
    //          | <LIST1> '&' <NEWLINE-LIST> <LIST1>
    //          | <LIST1> ';' <NEWLINE-LIST> <LIST1>
    //          | <LIST1> '\n' <NEWLINE-LIST> <LIST1>
    //          | <PIPELINE-COMMAND>
    fn parse_list1(&mut self) -> Result<Command, String> {
        let mut left = self.parse_pipeline_command()?;

        loop {
            let separator = if self.check(&TokenKind::And) {
                self.advance();
                self.skip_newlines();
                Separator::And
            } else if self.check(&TokenKind::Or) {
                self.advance();
                self.skip_newlines();
                Separator::Or
            } else if self.check(&TokenKind::Ampersand) {
                self.advance();
                self.skip_newlines();
                Separator::Background
            } else if self.check(&TokenKind::Semicolon) {
                self.advance();
                self.skip_newlines();
                Separator::Sequential
            } else if self.check(&TokenKind::Newline) {
                self.advance();
                self.skip_newlines();
                Separator::Sequential
            } else {
                break;
            };

            let right = self.parse_pipeline_command()?;

            left = Command::List(List {
                items: vec![
                    ListItem {
                        command: left,
                        separator: separator.clone(),
                    },
                    ListItem {
                        command: right,
                        separator: Separator::Sequential,
                    },
                ],
            });
        }

        Ok(left)
    }

    // <PIPELINE-COMMAND> ::= <PIPELINE>
    //                     | '!' <PIPELINE>
    fn parse_pipeline_command(&mut self) -> Result<Command, String> {
        let negated = if self.check(&TokenKind::Not) {
            self.advance();
            true
        } else {
            false
        };

        let pipeline = self.parse_pipeline()?;

        if negated {
            if let Command::Pipeline(mut p) = pipeline {
                p.negated = true;
                Ok(Command::Pipeline(p))
            } else {
                Ok(Command::Pipeline(Pipeline {
                    negated: true,
                    commands: vec![pipeline],
                }))
            }
        } else {
            Ok(pipeline)
        }
    }

    // <PIPELINE> ::= <PIPELINE> '|' <NEWLINE-LIST> <PIPELINE>
    //             | <COMMAND>
    fn parse_pipeline(&mut self) -> Result<Command, String> {
        let mut commands = vec![self.parse_command()?];

        while self.check(&TokenKind::Pipe) {
            self.advance();
            self.skip_newlines();
            commands.push(self.parse_command()?);
        }

        if commands.len() == 1 {
            Ok(commands.into_iter().next().unwrap())
        } else {
            Ok(Command::Pipeline(Pipeline {
                negated: false,
                commands,
            }))
        }
    }

    // <COMMAND> ::= <SIMPLE-COMMAND>
    //            | <SHELL-COMMAND>
    //            | <SHELL-COMMAND> <REDIRECTION-LIST>
    fn parse_command(&mut self) -> Result<Command, String> {
        if self.check(&TokenKind::If) {
            self.parse_if_command()
        } else if self.check(&TokenKind::While) {
            self.parse_while_command()
        } else if self.check(&TokenKind::Until) {
            self.parse_until_command()
        } else if self.check(&TokenKind::For) {
            self.parse_for_command()
        } else if self.check(&TokenKind::Case) {
            self.parse_case_command()
        } else if self.check(&TokenKind::LeftParen) {
            self.parse_subshell()
        } else if self.check(&TokenKind::LeftBrace) {
            self.parse_group_command()
        } else if self.check(&TokenKind::Function) {
            self.parse_function_def()
        } else {
            self.parse_simple_command()
        }
    }

    fn parse_simple_command(&mut self) -> Result<Command, String> {
        let mut cmd = SimpleCommand::new();
        let mut made_progress = false;

        loop {
            let old_pos = self.position;

            if self.check(&TokenKind::AssignmentWord) {
                let token = self.advance();
                if let Some((name, value)) = token.value.split_once('=') {
                    cmd.assignments.push(Assignment {
                        name: name.to_string(),
                        value: value.to_string(),
                    });
                }
                made_progress = true;
            } else if self.is_redirection() {
                cmd.redirections.push(self.parse_redirection()?);
                made_progress = true;
            } else if self.is_word_or_keyword() {
                // Accept both Word tokens and reserved words as arguments
                let token = self.advance();
                cmd.words.push(Word {
                    value: token.value.clone(),
                });
                made_progress = true;
            } else {
                break;
            }

            // Safety check: ensure we're making progress
            if self.position == old_pos {
                break;
            }
        }

        if !made_progress && cmd.assignments.is_empty() && cmd.words.is_empty() && cmd.redirections.is_empty() {
            return Err("Expected command".to_string());
        }

        Ok(Command::Simple(cmd))
    }

    fn is_word_or_keyword(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Word
                | TokenKind::Done
                | TokenKind::Time
                | TokenKind::In
                // Note: We don't include structural keywords like if, then, fi, etc.
                // as they should only appear in their grammatical positions
        )
    }

    fn parse_redirection(&mut self) -> Result<Redirection, String> {
        let fd = if self.check(&TokenKind::Number) {
            let token = self.advance();
            Some(token.value.parse::<i32>().unwrap())
        } else {
            None
        };

        let kind_token = self.current();
        let kind = match kind_token.kind {
            TokenKind::Greater => RedirectionKind::Output,
            TokenKind::Less => RedirectionKind::Input,
            TokenKind::GreatGreat => RedirectionKind::Append,
            TokenKind::LessLess => RedirectionKind::Heredoc,
            TokenKind::LessLessDash => RedirectionKind::HeredocStrip,
            TokenKind::LessAnd => RedirectionKind::InputDup,
            TokenKind::GreatAnd => RedirectionKind::OutputDup,
            TokenKind::LessGreat => RedirectionKind::InputOutput,
            TokenKind::GreatPipe => RedirectionKind::Clobber,
            TokenKind::AndGreat => RedirectionKind::OutputBoth,
            _ => return Err(format!("Expected redirection operator, got {:?}", kind_token)),
        };

        self.advance();

        let target = if self.check(&TokenKind::Dash) {
            self.advance();
            RedirectionTarget::Close
        } else if self.check(&TokenKind::Number) {
            let token = self.advance();
            RedirectionTarget::Fd(token.value.parse::<i32>().unwrap())
        } else if self.check(&TokenKind::Word) {
            let token = self.advance();
            RedirectionTarget::File(token.value.clone())
        } else {
            return Err("Expected redirection target".to_string());
        };

        Ok(Redirection { kind, fd, target })
    }

    fn parse_if_command(&mut self) -> Result<Command, String> {
        self.expect(&TokenKind::If)?;
        self.skip_newlines();

        let condition = Box::new(self.parse_compound_list(&[TokenKind::Then])?);

        self.expect(&TokenKind::Then)?;
        self.skip_newlines();

        let then_part = Box::new(self.parse_compound_list(&[
            TokenKind::Elif,
            TokenKind::Else,
            TokenKind::Fi,
        ])?);

        let mut elif_parts = Vec::new();
        while self.check(&TokenKind::Elif) {
            self.advance();
            self.skip_newlines();
            let elif_cond = self.parse_compound_list(&[TokenKind::Then])?;
            self.expect(&TokenKind::Then)?;
            self.skip_newlines();
            let elif_body = self.parse_compound_list(&[
                TokenKind::Elif,
                TokenKind::Else,
                TokenKind::Fi,
            ])?;
            elif_parts.push((elif_cond, elif_body));
        }

        let else_part = if self.check(&TokenKind::Else) {
            self.advance();
            self.skip_newlines();
            Some(Box::new(self.parse_compound_list(&[TokenKind::Fi])?))
        } else {
            None
        };

        self.expect(&TokenKind::Fi)?;

        Ok(Command::If(IfCommand {
            condition,
            then_part,
            elif_parts,
            else_part,
        }))
    }

    // Parse compound_list with specific terminators
    // This is similar to parse_list1 but stops at terminators
    fn parse_compound_list(&mut self, terminators: &[TokenKind]) -> Result<Command, String> {
        self.skip_newlines();

        let mut left = self.parse_pipeline_command()?;

        loop {
            // Check for terminators before consuming separators
            if terminators.iter().any(|t| self.check(t)) {
                break;
            }

            let separator = if self.check(&TokenKind::And) {
                self.advance();
                self.skip_newlines();
                Separator::And
            } else if self.check(&TokenKind::Or) {
                self.advance();
                self.skip_newlines();
                Separator::Or
            } else if self.check(&TokenKind::Ampersand) {
                self.advance();
                self.skip_newlines();
                Separator::Background
            } else if self.check(&TokenKind::Semicolon) {
                self.advance();
                self.skip_newlines();
                Separator::Sequential
            } else if self.check(&TokenKind::Newline) {
                self.advance();
                self.skip_newlines();
                Separator::Sequential
            } else {
                break;
            };

            // Check for terminators after consuming separator
            if terminators.iter().any(|t| self.check(t)) {
                // We consumed a separator but hit a terminator, that's OK
                // Return what we have so far
                break;
            }

            let right = self.parse_pipeline_command()?;

            left = Command::List(List {
                items: vec![
                    ListItem {
                        command: left,
                        separator: separator.clone(),
                    },
                    ListItem {
                        command: right,
                        separator: Separator::Sequential,
                    },
                ],
            });
        }

        Ok(left)
    }

    fn parse_while_command(&mut self) -> Result<Command, String> {
        self.expect(&TokenKind::While)?;
        self.skip_newlines();

        let condition = Box::new(self.parse_compound_list(&[TokenKind::Do])?);

        self.expect(&TokenKind::Do)?;
        self.skip_newlines();

        let body = Box::new(self.parse_compound_list(&[TokenKind::Done])?);

        self.expect(&TokenKind::Done)?;

        Ok(Command::While(WhileCommand { condition, body }))
    }

    fn parse_until_command(&mut self) -> Result<Command, String> {
        self.expect(&TokenKind::Until)?;
        self.skip_newlines();

        let condition = Box::new(self.parse_compound_list(&[TokenKind::Do])?);

        self.expect(&TokenKind::Do)?;
        self.skip_newlines();

        let body = Box::new(self.parse_compound_list(&[TokenKind::Done])?);

        self.expect(&TokenKind::Done)?;

        Ok(Command::Until(UntilCommand { condition, body }))
    }

    fn parse_for_command(&mut self) -> Result<Command, String> {
        self.expect(&TokenKind::For)?;

        let var_token = self.expect(&TokenKind::Word)?;
        let variable = var_token.value.clone();

        self.skip_newlines();

        let words = if self.check(&TokenKind::In) {
            self.advance();
            let mut words = Vec::new();
            while self.check(&TokenKind::Word) {
                words.push(self.advance().value.clone());
            }
            words
        } else {
            Vec::new()
        };

        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }
        self.skip_newlines();

        self.expect(&TokenKind::Do)?;
        self.skip_newlines();

        let body = Box::new(self.parse_compound_list(&[TokenKind::Done])?);

        self.expect(&TokenKind::Done)?;

        Ok(Command::For(ForCommand {
            variable,
            words,
            body,
        }))
    }

    fn parse_case_command(&mut self) -> Result<Command, String> {
        self.expect(&TokenKind::Case)?;

        let word_token = self.expect(&TokenKind::Word)?;
        let word = word_token.value.clone();

        self.skip_newlines();
        self.expect(&TokenKind::In)?;
        self.skip_newlines();

        let mut cases = Vec::new();

        while !self.check(&TokenKind::Esac) {
            let mut patterns = Vec::new();

            if self.check(&TokenKind::LeftParen) {
                self.advance();
            }

            loop {
                let pattern = self.expect(&TokenKind::Word)?;
                patterns.push(pattern.value.clone());

                if self.check(&TokenKind::Pipe) {
                    self.advance();
                } else {
                    break;
                }
            }

            self.expect(&TokenKind::RightParen)?;
            self.skip_newlines();

            let body = Box::new(self.parse_list()?);

            cases.push(CaseClause { patterns, body });

            if self.check(&TokenKind::Semicolon) {
                self.advance();
                self.advance(); // ;;
            }
            self.skip_newlines();
        }

        self.expect(&TokenKind::Esac)?;

        Ok(Command::Case(CaseCommand { word, cases }))
    }

    fn parse_subshell(&mut self) -> Result<Command, String> {
        self.expect(&TokenKind::LeftParen)?;
        self.skip_newlines();

        let command = self.parse_list()?;

        self.expect(&TokenKind::RightParen)?;

        Ok(Command::Subshell(Box::new(command)))
    }

    fn parse_group_command(&mut self) -> Result<Command, String> {
        self.expect(&TokenKind::LeftBrace)?;
        self.skip_newlines();

        let mut commands = Vec::new();

        while !self.check(&TokenKind::RightBrace) {
            commands.push(self.parse_list()?);
            self.skip_newlines();
        }

        self.expect(&TokenKind::RightBrace)?;

        Ok(Command::Group(commands))
    }

    fn parse_function_def(&mut self) -> Result<Command, String> {
        self.expect(&TokenKind::Function)?;

        let name_token = self.expect(&TokenKind::Word)?;
        let name = name_token.value.clone();

        if self.check(&TokenKind::LeftParen) {
            self.advance();
            self.expect(&TokenKind::RightParen)?;
        }

        self.skip_newlines();

        let body = Box::new(self.parse_group_command()?);

        Ok(Command::FunctionDef(FunctionDef { name, body }))
    }

    fn is_redirection(&self) -> bool {
        if self.check(&TokenKind::Number) {
            if self.position + 1 < self.tokens.len() {
                let next = &self.tokens[self.position + 1];
                matches!(
                    next.kind,
                    TokenKind::Greater
                        | TokenKind::Less
                        | TokenKind::GreatGreat
                        | TokenKind::LessLess
                        | TokenKind::LessAnd
                        | TokenKind::GreatAnd
                        | TokenKind::LessGreat
                )
            } else {
                false
            }
        } else {
            matches!(
                self.current().kind,
                TokenKind::Greater
                    | TokenKind::Less
                    | TokenKind::GreatGreat
                    | TokenKind::LessLess
                    | TokenKind::LessAnd
                    | TokenKind::GreatAnd
                    | TokenKind::LessGreat
                    | TokenKind::GreatPipe
                    | TokenKind::AndGreat
                    | TokenKind::LessLessDash
            )
        }
    }

    fn skip_newlines(&mut self) {
        while self.check(&TokenKind::Newline) {
            self.advance();
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        !self.is_at_end() && &self.current().kind == kind
    }

    fn current(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.position += 1;
        }
        &self.tokens[self.position - 1]
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<&Token, String> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(format!(
                "Expected {:?}, got {:?} at {}:{}",
                kind,
                self.current().kind,
                self.current().position.line,
                self.current().position.column
            ))
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len() || self.current().kind == TokenKind::Eof
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn test_simple_command() {
        let mut lexer = Lexer::new("ls -la");
        let tokens = lexer.tokenize().unwrap();
        eprintln!("Tokens: {:?}", tokens);
        let mut parser = Parser::new(tokens);
        let commands = parser.parse();
        eprintln!("Parse result: {:?}", commands);
        assert!(commands.is_ok(), "Parse failed: {:?}", commands.err());
        let commands = commands.unwrap();
        assert_eq!(commands.len(), 1);
    }

    #[test]
    fn test_pipeline() {
        let mut lexer = Lexer::new("cat file | grep foo");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let commands = parser.parse().unwrap();
        assert!(matches!(commands[0], Command::Pipeline(_)));
    }

    #[test]
    fn test_if_command() {
        let mut lexer = Lexer::new("if true; then echo yes; fi");
        let tokens = lexer.tokenize().unwrap();
        eprintln!("Tokens: {:?}", tokens);
        let mut parser = Parser::new(tokens);
        let commands = parser.parse();
        eprintln!("Parse result: {:?}", commands);
        assert!(commands.is_ok(), "Parse failed: {:?}", commands.err());
        let commands = commands.unwrap();
        assert!(matches!(commands[0], Command::If(_)));
    }

    #[test]
    fn test_list1_with_and_operator() {
        // Test: cmd1 && cmd2
        let mut lexer = Lexer::new("true && echo success");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], Command::List(_)));
    }

    #[test]
    fn test_list1_with_or_operator() {
        // Test: cmd1 || cmd2
        let mut lexer = Lexer::new("false || echo fallback");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], Command::List(_)));
    }

    #[test]
    fn test_list1_with_semicolon() {
        // Test: cmd1 ; cmd2 ; cmd3
        let mut lexer = Lexer::new("echo a ; echo b ; echo c");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], Command::List(_)));
    }

    #[test]
    fn test_list1_with_background() {
        // Test: cmd1 & cmd2
        let mut lexer = Lexer::new("sleep 1 & echo done");
        let tokens = lexer.tokenize().unwrap();
        eprintln!("Tokens: {:?}", tokens);
        let mut parser = Parser::new(tokens);
        let commands = parser.parse();
        eprintln!("Parse result: {:?}", commands);
        assert!(commands.is_ok(), "Parse failed: {:?}", commands.err());
        let commands = commands.unwrap();
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], Command::List(_)));
    }

    #[test]
    fn test_compound_list_in_if() {
        // Test compound_list with multiple commands
        let mut lexer = Lexer::new("if true; then echo a; echo b; fi");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], Command::If(_)));
    }

    #[test]
    fn test_compound_list_with_and_in_if() {
        // Test compound_list with && inside if
        let mut lexer = Lexer::new("if true && false; then echo no; fi");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], Command::If(_)));
    }
}
