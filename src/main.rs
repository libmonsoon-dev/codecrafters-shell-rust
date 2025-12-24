use std::io::{self, Write};

fn main() {
    let mut shell = Shell::new();
    shell.repl();
}

struct Shell {
    input: io::Stdin,
    output: io::Stdout,
    command: String,
}

impl Shell {
    fn new() -> Shell {
        Shell {
            input: io::stdin(),
            output: io::stdout(),
            command: String::new(),
        }
    }

    fn read(&mut self) -> io::Result<()> {
        self.output.write_fmt(format_args!("$ "))?;
        self.output.flush()?;
        Ok(())
    }

    fn eval(&mut self) -> io::Result<()> {
        self.input.read_line(&mut self.command)?;
        Ok(())
    }

    fn print(&mut self) -> io::Result<()> {
        self.output
            .write_fmt(format_args!("{}: command not found\n", self.command.trim()))?;
        self.command.clear();
        Ok(())
    }

    fn repl(&mut self) {
        loop {
            self.read().unwrap();
            self.eval().unwrap();
            self.print().unwrap();
        }
    }
}
