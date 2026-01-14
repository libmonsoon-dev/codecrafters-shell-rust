pub mod bin_path;
pub mod completion;
pub mod editor;
pub mod lexer;
pub mod macros;
pub mod parser;
pub mod pipeline;
pub mod shell;

pub static BUILTIN_COMMANDS: &[&str] = &["exit", "echo", "type", "pwd", "cd", "history"];

#[derive(thiserror::Error, Debug)]
pub enum CallError {
    #[error("exit")]
    Exit,
}
