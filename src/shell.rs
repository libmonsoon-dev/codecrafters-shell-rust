use crate::parser::{OutputStream, Parser, Redirect};
use crate::print;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Once;
use std::thread;

static BUILTIN_COMMANDS: &[&str] = &["exit", "echo", "type", "pwd", "cd"];

pub struct Shell {
    input_buffer: String,
    command: Vec<String>,
    env_once: Once,
    path: Vec<String>,
    redirects: Vec<Redirect>,
}

impl Shell {
    pub fn new() -> Shell {
        Shell {
            input_buffer: String::new(),
            command: Vec::new(),
            env_once: Once::new(),
            path: Vec::new(),
            redirects: Vec::new(),
        }
    }

    fn read(&mut self) -> io::Result<()> {
        self.input_buffer.clear();
        io::stdin().read_line(&mut self.input_buffer)?;

        //TODO: pass this vectors to parser to avoid allocations
        (self.command, self.redirects) = Parser::new(self.input_buffer.clone()).parse();

        Ok(())
    }

    fn eval(&mut self) -> io::Result<()> {
        if self.command.is_empty() {
            return Ok(());
        }

        if BUILTIN_COMMANDS.contains(&self.command[0].as_ref()) {
            match self.command[0].as_ref() {
                "exit" => process::exit(0),
                "echo" => self.echo_builtin()?,
                "type" => self.type_builtin()?,
                "pwd" => print!("{}\n", env::current_dir()?.display()),
                "cd" => self.cd_builtin()?,
                _ => unimplemented!("builtin command {}", self.command[0]),
            }

            return Ok(());
        }

        if let Some(_) = self.lookup_path(self.command[0].clone())? {
            let mut cmd = process::Command::new(self.command[0].clone());

            self.command[1..].iter().for_each(|arg| {
                cmd.arg(arg);
            });

            let mut child = cmd
                .stdout(process::Stdio::piped())
                .stderr(process::Stdio::piped())
                .spawn()?;

            let mut child_stdout = child.stdout.take().expect("handle present");
            let mut output = self.get_output()?;
            let stdout_thread = thread::spawn(move || {
                io::copy(&mut child_stdout, &mut output).unwrap();
            });

            let mut child_stderr = child.stderr.take().expect("handle present");
            let mut errors = self.get_error_output()?;
            let stderr_thread = thread::spawn(move || {
                io::copy(&mut child_stderr, &mut errors).unwrap();
            });

            child.wait()?;
            stdout_thread.join().unwrap();
            stderr_thread.join().unwrap();

            return Ok(());
        }

        print!("{}: command not found\n", self.command[0].trim());
        Ok(())
    }

    fn print(&mut self) -> io::Result<()> {
        print!("$ ");
        io::stdout().flush()?;

        Ok(())
    }

    pub fn repl(&mut self) {
        self.print().unwrap();

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
                    print!("{} is a shell builtin\n", arg);
                    return Ok(());
                }

                if let Some(path) = self.lookup_path(arg.clone())? {
                    print!("{} is {}\n", arg, path.display());
                    return Ok(());
                }

                print!("{}: not found\n", arg);

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
            print!("cd: {path}: No such file or directory\n");
            return Ok(());
        }

        env::set_current_dir(path)?;

        Ok(())
    }

    fn echo_builtin(&mut self) -> io::Result<()> {
        let str = self.command[1..].join(" ");
        self.get_output()?.write_fmt(format_args!("{str}\n"))?;

        Ok(())
    }

    fn get_output(&mut self) -> io::Result<Box<dyn io::Write + Send>> {
        let Some(redirect) = self
            .redirects
            .iter()
            .find(|r| r.from == OutputStream::Stdout)
        else {
            return Ok(Box::new(io::stdout()));
        };

        let file = redirect.open_output()?;
        Ok(Box::new(file))
    }

    fn get_error_output(&mut self) -> io::Result<Box<dyn io::Write + Send>> {
        let Some(redirect) = self
            .redirects
            .iter()
            .find(|r| r.from == OutputStream::Stderr)
        else {
            return Ok(Box::new(io::stderr()));
        };

        let file = redirect.open_output()?;
        Ok(Box::new(file))
    }
}
