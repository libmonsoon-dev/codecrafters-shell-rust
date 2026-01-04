use codecrafters_shell::shell::Shell;

fn main() -> anyhow::Result<()> {
    let mut shell = Shell::new()?;
    shell.repl()?;

    Ok(())
}
