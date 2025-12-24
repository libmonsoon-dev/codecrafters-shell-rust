use std::env;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::process::Command;
use std::sync::Once;

macro_rules! print {
    ($self:expr, $fmt:expr) => {{
        $self.output.write_fmt(format_args!($fmt))?;
    }};
    ($self:expr, $fmt:expr, $($args:tt)*) => {{
        $self.output.write_fmt(format_args!($fmt, $($args)*))?;
    }};
}

static BUILTIN_COMMANDS: &[&str] = &["exit", "echo", "type", "pwd", "cd"];

fn main() {
    let mut shell = Shell::new();
    shell.repl();
}

struct Shell {
    input: io::Stdin,
    output: io::Stdout,
    input_buffer: String,
    command: Vec<String>,
    env_once: Once,
    path: Vec<String>,
}

impl Shell {
    fn new() -> Shell {
        Shell {
            input: io::stdin(),
            output: io::stdout(),
            input_buffer: String::new(),
            command: Vec::new(),
            env_once: Once::new(),
            path: Vec::new(),
        }
    }

    fn read(&mut self) -> io::Result<()> {
        print!(self, "$ ");
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
        if BUILTIN_COMMANDS.contains(&self.command[0].as_ref()) {
            match self.command[0].as_ref() {
                "exit" => exit(0),
                "echo" => print!(self, "{}\n", self.command[1..].join(" ")),
                "type" => self.type_builtin()?,
                "pwd" => print!(self, "{}\n", env::current_dir()?.display()),
                "cd" => self.cd_builtin()?,
                _ => unimplemented!("builtin command {}", self.command[0]),
            }

            return Ok(());
        }

        if let Some(_) = self.lookup_path(self.command[0].clone())? {
            let mut cmd = Command::new(self.command[0].clone());

            self.command[1..].iter().for_each(|arg| {
                cmd.arg(arg);
            });

            // TODO: redirect stdout to self.output
            cmd.spawn()?.wait()?;

            return Ok(());
        }

        print!(self, "{}: command not found\n", self.command[0].trim());
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
        let _ = &self.command.clone()[1..]
            .iter()
            .try_for_each(|arg| -> io::Result<()> {
                if BUILTIN_COMMANDS.contains(&arg.as_str()) {
                    print!(self, "{} is a shell builtin\n", arg);
                    return Ok(());
                }

                if let Some(path) = self.lookup_path(arg.clone())? {
                    print!(self, "{} is {}\n", arg, path.display());
                    return Ok(());
                }

                print!(self, "{}: not found\n", arg);

                Ok(())
            })?;

        Ok(())
    }

    fn lookup_path(&mut self, bin: String) -> io::Result<Option<PathBuf>> {
        self.load_path();
        for dir in self.path.clone() {
            let path = Path::new(&dir).join(bin.clone());
            let result = fs::metadata(path.clone());
            if matches!(result, Err(ref err) if err.kind() == io::ErrorKind::NotFound) {
                continue;
            }

            let attr = result?;
            //TODO: handle user and group permissions
            if attr.permissions().mode() & 0o001 != 0 {
                return Ok(Some(path));
            }
        }

        Ok(None)
    }

    fn load_path(&mut self) {
        self.env_once.call_once(|| {
            self.path = env::var("PATH")
                .unwrap()
                .split(':')
                .map(String::from)
                .collect();
        })
    }

    fn cd_builtin(&mut self) -> io::Result<()> {
        let path = if self.command.len() == 1 || self.command[1] == "~" {
            env::var("HOME").unwrap()
        } else {
            self.command[1].clone()
        };
        let attr = fs::metadata(path.clone());
        if matches!(attr, Err(ref err) if err.kind() == io::ErrorKind::NotFound) {
            print!(self, "cd: {path}: No such file or directory\n");
            return Ok(());
        }

        env::set_current_dir(path)?;

        Ok(())
    }
}
