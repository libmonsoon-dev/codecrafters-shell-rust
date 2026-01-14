use codecrafters_shell::shell::{contain, Shell};
use codecrafters_shell::CallError;

fn main() -> anyhow::Result<()> {
    let mut shell = Shell::new()?;
    match shell.repl() {
        Ok(_) => Ok(()),
        Err(err) if contain::<CallError>(err.chain()) => Ok(()),
        Err(err) => Err(err.into()),
    }
}
