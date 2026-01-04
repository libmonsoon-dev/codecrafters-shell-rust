pub mod completion;
pub mod lexer;
pub mod macros;
pub mod parser;
pub mod read_line;
pub mod shell;

pub static BUILTIN_COMMANDS: &[&str] = &["exit", "echo", "type", "pwd", "cd"];
