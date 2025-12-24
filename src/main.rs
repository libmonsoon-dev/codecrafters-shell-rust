use std::io::{self, Write};
use std::process::exit;

fn main() {
    let mut shell = Shell::new();
    shell.repl();
}

struct Shell {
    input: io::Stdin,
    output: io::Stdout,
    input_buffer: String,
    command: Vec<String>,
}

impl Shell {
    fn new() -> Shell {
        Shell {
            input: io::stdin(),
            output: io::stdout(),
            input_buffer: String::new(),
            command: Vec::new(),
        }
    }

    fn read(&mut self) -> io::Result<()> {
        self.output.write_fmt(format_args!("$ "))?;
        self.output.flush()?;

        self.input_buffer.clear();
        self.input.read_line(&mut self.input_buffer)?;

        self.command = self
            .input_buffer
            .split_whitespace()
            .map(|str| str.trim().to_owned())
            .collect();

        Ok(())
    }

    fn eval(&mut self) -> io::Result<()> {
        match self.command[0].trim() {
            "exit" => exit(0),
            "echo" => self
                .output
                .write_fmt(format_args!("{}\n", self.command[1..].join(" ")))?,
            "type" => self.type_builtin()?,
            _ => self.output.write_fmt(format_args!(
                "{}: command not found\n",
                self.command[0].trim()
            ))?,
        }

        Ok(())
    }

    fn print(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn repl(&mut self) {
        loop {
            self.read().unwrap();
            self.eval().unwrap();
            self.print().unwrap();
        }
    }

    fn type_builtin(&mut self) -> io::Result<()> {
        for arg in &self.command[1..] {
            match arg.as_str() {
                "exit" | "echo" | "type" => self
                    .output
                    .write_fmt(format_args!("{} is a shell builtin\n", arg))?,
                _ => self
                    .output
                    .write_fmt(format_args!("{}: not found\n", arg))?,
            }
        }

        Ok(())
    }
}
