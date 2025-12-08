use crate::token::{Position, Token, TokenKind};

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        while !self.is_eof() {
            self.skip_whitespace();
            if self.is_eof() {
                break;
            }

            let token = self.next_token()?;
            tokens.push(token);
        }

        tokens.push(Token::new(
            TokenKind::Eof,
            String::new(),
            Position::new(self.line, self.column),
        ));

        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token, String> {
        let pos = Position::new(self.line, self.column);
        let ch = self.current_char();

        match ch {
            '\n' => {
                self.advance();
                Ok(Token::new(TokenKind::Newline, "\n".to_string(), pos))
            }
            ';' => {
                self.advance();
                if self.current_char() == ';' {
                    self.advance();
                    Ok(Token::new(TokenKind::DoubleSemicolon, ";;".to_string(), pos))
                } else {
                    Ok(Token::new(TokenKind::Semicolon, ";".to_string(), pos))
                }
            }
            '|' => {
                self.advance();
                if self.current_char() == '|' {
                    self.advance();
                    Ok(Token::new(TokenKind::Or, "||".to_string(), pos))
                } else {
                    Ok(Token::new(TokenKind::Pipe, "|".to_string(), pos))
                }
            }
            '&' => {
                self.advance();
                if self.current_char() == '&' {
                    self.advance();
                    Ok(Token::new(TokenKind::And, "&&".to_string(), pos))
                } else if self.current_char() == '>' {
                    self.advance();
                    Ok(Token::new(TokenKind::AndGreat, "&>".to_string(), pos))
                } else {
                    Ok(Token::new(TokenKind::Ampersand, "&".to_string(), pos))
                }
            }
            '>' => {
                self.advance();
                if self.current_char() == '>' {
                    self.advance();
                    Ok(Token::new(TokenKind::GreatGreat, ">>".to_string(), pos))
                } else if self.current_char() == '&' {
                    self.advance();
                    Ok(Token::new(TokenKind::GreatAnd, ">&".to_string(), pos))
                } else if self.current_char() == '|' {
                    self.advance();
                    Ok(Token::new(TokenKind::GreatPipe, ">|".to_string(), pos))
                } else {
                    Ok(Token::new(TokenKind::Greater, ">".to_string(), pos))
                }
            }
            '<' => {
                self.advance();
                if self.current_char() == '<' {
                    self.advance();
                    if self.current_char() == '-' {
                        self.advance();
                        Ok(Token::new(TokenKind::LessLessDash, "<<-".to_string(), pos))
                    } else {
                        Ok(Token::new(TokenKind::LessLess, "<<".to_string(), pos))
                    }
                } else if self.current_char() == '&' {
                    self.advance();
                    Ok(Token::new(TokenKind::LessAnd, "<&".to_string(), pos))
                } else if self.current_char() == '>' {
                    self.advance();
                    Ok(Token::new(TokenKind::LessGreat, "<>".to_string(), pos))
                } else {
                    Ok(Token::new(TokenKind::Less, "<".to_string(), pos))
                }
            }
            '!' => {
                self.advance();
                Ok(Token::new(TokenKind::Not, "!".to_string(), pos))
            }
            '(' => {
                self.advance();
                Ok(Token::new(TokenKind::LeftParen, "(".to_string(), pos))
            }
            ')' => {
                self.advance();
                Ok(Token::new(TokenKind::RightParen, ")".to_string(), pos))
            }
            '{' => {
                self.advance();
                Ok(Token::new(TokenKind::LeftBrace, "{".to_string(), pos))
            }
            '}' => {
                self.advance();
                Ok(Token::new(TokenKind::RightBrace, "}".to_string(), pos))
            }
            '-' if self.is_standalone_dash() => {
                self.advance();
                Ok(Token::new(TokenKind::Dash, "-".to_string(), pos))
            }
            '#' => {
                // Comment: skip until newline
                while !self.is_eof() && self.current_char() != '\n' {
                    self.advance();
                }
                self.next_token()
            }
            '"' => self.read_quoted_string('"'),
            '\'' => self.read_quoted_string('\''),
            '$' => self.read_variable_or_word(pos),
            _ if ch.is_ascii_digit() => self.read_number_or_word(pos),
            _ if self.is_word_start(ch) => self.read_word(pos),
            _ => Err(format!(
                "Unexpected character '{}' at {}:{}",
                ch, self.line, self.column
            )),
        }
    }

    fn read_variable_or_word(&mut self, pos: Position) -> Result<Token, String> {
        let mut word = String::new();

        // Start with $
        word.push(self.current_char());
        self.advance();

        // Check for ${VAR} syntax
        if !self.is_eof() && self.current_char() == '{' {
            word.push(self.current_char());
            self.advance();

            while !self.is_eof() && self.current_char() != '}' {
                word.push(self.current_char());
                self.advance();
            }

            if self.is_eof() {
                return Err("Unclosed variable expansion".to_string());
            }

            word.push(self.current_char()); // closing }
            self.advance();
        } else {
            // $VAR syntax - read variable name
            while !self.is_eof() && (self.current_char().is_alphanumeric() || self.current_char() == '_') {
                word.push(self.current_char());
                self.advance();
            }
        }

        Ok(Token::new(TokenKind::Word, word, pos))
    }

    fn read_word(&mut self, pos: Position) -> Result<Token, String> {
        let mut word = String::new();

        while !self.is_eof() && self.is_word_char(self.current_char()) {
            word.push(self.current_char());
            self.advance();
        }

        // Check if it contains '='
        if !self.is_eof() && self.current_char() == '=' {
            word.push('=');
            self.advance();

            // Read the value part (which might be quoted)
            if !self.is_eof() {
                if self.current_char() == '"' || self.current_char() == '\'' {
                    // Read quoted value
                    let quote = self.current_char();
                    self.advance(); // Skip opening quote

                    while !self.is_eof() && self.current_char() != quote {
                        if self.current_char() == '\\' && quote == '"' {
                            self.advance();
                            if !self.is_eof() {
                                word.push(self.current_char());
                                self.advance();
                            }
                        } else {
                            word.push(self.current_char());
                            self.advance();
                        }
                    }

                    if !self.is_eof() {
                        self.advance(); // Skip closing quote
                    }
                } else {
                    // Read unquoted value
                    while !self.is_eof() && self.is_word_char(self.current_char()) {
                        word.push(self.current_char());
                        self.advance();
                    }
                }
            }

            return Ok(Token::new(TokenKind::AssignmentWord, word, pos));
        }

        // Check if it's a keyword
        let kind = match word.as_str() {
            "if" => TokenKind::If,
            "then" => TokenKind::Then,
            "else" => TokenKind::Else,
            "elif" => TokenKind::Elif,
            "fi" => TokenKind::Fi,
            "case" => TokenKind::Case,
            "esac" => TokenKind::Esac,
            "for" => TokenKind::For,
            "select" => TokenKind::Select,
            "while" => TokenKind::While,
            "until" => TokenKind::Until,
            "do" => TokenKind::Do,
            "done" => TokenKind::Done,
            "in" => TokenKind::In,
            "function" => TokenKind::Function,
            "time" => TokenKind::Time,
            _ => TokenKind::Word,
        };

        Ok(Token::new(kind, word, pos))
    }

    fn read_number_or_word(&mut self, pos: Position) -> Result<Token, String> {
        let mut value = String::new();

        while !self.is_eof() && self.current_char().is_ascii_digit() {
            value.push(self.current_char());
            self.advance();
        }

        // Check if it's followed by redirection
        let next_ch = self.current_char();
        if next_ch == '>' || next_ch == '<' {
            return Ok(Token::new(TokenKind::Number, value, pos));
        }

        // Otherwise, continue reading as a word
        while !self.is_eof() && self.is_word_char(self.current_char()) {
            value.push(self.current_char());
            self.advance();
        }

        // Check for assignment
        if !self.is_eof() && self.current_char() == '=' {
            value.push('=');
            self.advance();
            while !self.is_eof() && self.is_word_char(self.current_char()) {
                value.push(self.current_char());
                self.advance();
            }
            return Ok(Token::new(TokenKind::AssignmentWord, value, pos));
        }

        Ok(Token::new(TokenKind::Word, value, pos))
    }

    fn read_quoted_string(&mut self, quote: char) -> Result<Token, String> {
        let pos = Position::new(self.line, self.column);
        let mut value = String::new();
        self.advance(); // Skip opening quote

        while !self.is_eof() && self.current_char() != quote {
            if self.current_char() == '\\' && quote == '"' {
                // Handle escape sequences in double quotes
                self.advance();
                if !self.is_eof() {
                    value.push(self.current_char());
                    self.advance();
                }
            } else {
                value.push(self.current_char());
                self.advance();
            }
        }

        if self.is_eof() {
            return Err(format!("Unterminated string at {}:{}", pos.line, pos.column));
        }

        self.advance(); // Skip closing quote

        Ok(Token::new(TokenKind::Word, value, pos))
    }

    fn is_word_start(&self, ch: char) -> bool {
        ch.is_alphabetic() || ch == '_' || ch == '-' || ch == '.' || ch == '/'
    }

    fn is_word_char(&self, ch: char) -> bool {
        ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '.' || ch == '/' || ch == '$'
    }

    fn is_standalone_dash(&self) -> bool {
        // Dash is standalone only if it's followed by whitespace, EOF, or redirection
        if self.position + 1 >= self.input.len() {
            return true;
        }
        let next = self.input[self.position + 1];
        next.is_whitespace() || next == '>' || next == '<' || next == '|' || next == '&' || next == ';'
    }

    fn skip_whitespace(&mut self) {
        while !self.is_eof() {
            let ch = self.current_char();
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn current_char(&self) -> char {
        if self.is_eof() {
            '\0'
        } else {
            self.input[self.position]
        }
    }

    fn advance(&mut self) {
        if !self.is_eof() {
            if self.input[self.position] == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.position += 1;
        }
    }

    fn is_eof(&self) -> bool {
        self.position >= self.input.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let mut lexer = Lexer::new("ls -la /tmp");
        let tokens = lexer.tokenize().unwrap();
        // ls, -la, /tmp, Newline, EOF (newline is added implicitly if not present)
        // Actually: ls, -la, /tmp, EOF
        assert!(tokens.len() >= 4); // at least ls, -la, /tmp, EOF
        assert_eq!(tokens[0].kind, TokenKind::Word);
        assert_eq!(tokens[0].value, "ls");
    }

    #[test]
    fn test_pipe() {
        let mut lexer = Lexer::new("cat file.txt | grep foo");
        let tokens = lexer.tokenize().unwrap();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Pipe));
    }

    #[test]
    fn test_redirection() {
        let mut lexer = Lexer::new("echo hello > file.txt");
        let tokens = lexer.tokenize().unwrap();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Greater));
    }

    #[test]
    fn test_assignment() {
        let mut lexer = Lexer::new("FOO=bar");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::AssignmentWord);
        assert_eq!(tokens[0].value, "FOO=bar");
    }
}
