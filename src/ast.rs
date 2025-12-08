use serde::Serialize;

/// Abstract Syntax Tree for shell commands
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Command {
    Simple(SimpleCommand),
    Pipeline(Pipeline),
    List(List),
    Subshell(Box<Command>),
    If(IfCommand),
    While(WhileCommand),
    Until(UntilCommand),
    For(ForCommand),
    Case(CaseCommand),
    FunctionDef(FunctionDef),
    Group(Vec<Command>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SimpleCommand {
    pub assignments: Vec<Assignment>,
    pub words: Vec<Word>,
    pub redirections: Vec<Redirection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Assignment {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Word {
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Redirection {
    pub kind: RedirectionKind,
    pub fd: Option<i32>,
    pub target: RedirectionTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RedirectionKind {
    Input,          // <
    Output,         // >
    Append,         // >>
    Heredoc,        // <<
    HeredocStrip,   // <<-
    InputDup,       // <&
    OutputDup,      // >&
    InputOutput,    // <>
    Clobber,        // >|
    OutputBoth,     // &>
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RedirectionTarget {
    File(String),
    Fd(i32),
    Close,          // &- or >&-
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pipeline {
    pub negated: bool,
    pub commands: Vec<Command>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct List {
    pub items: Vec<ListItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListItem {
    pub command: Command,
    pub separator: Separator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Separator {
    Sequential,     // ; or newline
    Background,     // &
    And,            // &&
    Or,             // ||
    Pipe,           // |
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IfCommand {
    pub condition: Box<Command>,
    pub then_part: Box<Command>,
    pub elif_parts: Vec<(Command, Command)>,
    pub else_part: Option<Box<Command>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WhileCommand {
    pub condition: Box<Command>,
    pub body: Box<Command>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UntilCommand {
    pub condition: Box<Command>,
    pub body: Box<Command>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ForCommand {
    pub variable: String,
    pub words: Vec<String>,
    pub body: Box<Command>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseCommand {
    pub word: String,
    pub cases: Vec<CaseClause>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseClause {
    pub patterns: Vec<String>,
    pub body: Box<Command>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FunctionDef {
    pub name: String,
    pub body: Box<Command>,
}

impl SimpleCommand {
    pub fn new() -> Self {
        Self {
            assignments: Vec::new(),
            words: Vec::new(),
            redirections: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.assignments.is_empty() && self.words.is_empty() && self.redirections.is_empty()
    }
}

impl Default for SimpleCommand {
    fn default() -> Self {
        Self::new()
    }
}
