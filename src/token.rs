#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
    pub position: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // Basic tokens
    Word,
    Number,
    AssignmentWord,

    // Operators
    Pipe,           // |
    And,            // &&
    Or,             // ||
    Semicolon,      // ;
    Ampersand,      // &
    Not,            // !

    // Redirections
    Greater,        // >
    Less,           // <
    GreatGreat,     // >>
    LessLess,       // <<
    LessAnd,        // <&
    GreatAnd,       // >&
    LessLessDash,   // <<-
    GreatPipe,      // >|
    AndGreat,       // &>
    LessGreat,      // <>

    // Parentheses and braces
    LeftParen,      // (
    RightParen,     // )
    LeftBrace,      // {
    RightBrace,     // }

    // Keywords
    If,
    Then,
    Else,
    Elif,
    Fi,
    Case,
    Esac,
    For,
    Select,
    While,
    Until,
    Do,
    Done,
    In,
    Function,
    Time,

    // Separators
    Newline,
    Dash,           // -

    // Special
    Eof,
}

impl Token {
    pub fn new(kind: TokenKind, value: String, position: Position) -> Self {
        Self {
            kind,
            value,
            position,
        }
    }
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}
