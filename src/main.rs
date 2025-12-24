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
}

impl Shell {
    fn new() -> Shell {
        Shell {
            input: io::stdin(),
            output: io::stdout(),
            input_buffer: String::new(),
        }
    }

    fn read(&mut self) -> io::Result<()> {
        self.output.write_fmt(format_args!("$ "))?;
        self.output.flush()?;
        Ok(())
    }

    fn eval(&mut self) -> io::Result<()> {
        self.input_buffer.clear();
        self.input.read_line(&mut self.input_buffer)?;

        let command = self.input_buffer.split_whitespace().collect::<Vec<_>>();

        match command[0].trim() {
            "exit" => exit(0),
            "echo" => self
                .output
                .write_fmt(format_args!("{}", command[1..].join(" ")))?,
            &_ => {}
        }

        Ok(())
    }

    fn print(&mut self) -> io::Result<()> {
        self.output.write_fmt(format_args!(
            "{}: command not found\n",
            self.input_buffer.trim()
        ))?;
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
