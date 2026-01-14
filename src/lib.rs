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
pub struct ExitError {}

impl std::fmt::Display for ExitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("ExitError"))
    }
}
