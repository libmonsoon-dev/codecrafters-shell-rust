use crate::bin_path::BinPath;
use crate::editor::Editor;
use crate::parser::{OutputStream, Parser, Redirect};
use crate::{print, BUILTIN_COMMANDS};
use std::cell::RefCell;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;
use std::rc::Rc;
use std::thread;

pub struct Shell {
    editor: Editor,
    input_buffer: String,
    command: Vec<String>,
    bin_path: Rc<RefCell<BinPath>>,
    redirects: Vec<Redirect>,
}

impl Shell {
    pub fn new() -> anyhow::Result<Shell> {
        let bin_path = Rc::new(RefCell::new(BinPath::new()));

        Ok(Shell {
            editor: Editor::new(bin_path.clone())?,
            input_buffer: String::new(),
            command: Vec::new(),
            bin_path,
            redirects: Vec::new(),
        })
    }

    fn read(&mut self) -> anyhow::Result<()> {
        self.input_buffer = self.editor.readline("$ ")?;

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

        let mut bin_path = self.bin_path.borrow_mut();
        if let Some(_) = bin_path.lookup(&self.command[0])? {
            drop(bin_path);

            let mut cmd = process::Command::new(&self.command[0]);

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

    pub fn repl(&mut self) -> anyhow::Result<()> {
        loop {
            self.read()?;
            self.eval()?;
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

                if let Some(path) = self.bin_path.borrow_mut().lookup(&arg)? {
                    print!("{} is {}\n", arg, path.display());
                    return Ok(());
                }

                print!("{}: not found\n", arg);

                Ok(())
            })?;

        Ok(())
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
        self.get_error_output()?; //create file if needed

        Ok(())
    }

    fn get_output(&mut self) -> io::Result<Box<dyn Write + Send>> {
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

    fn get_error_output(&mut self) -> io::Result<Box<dyn Write + Send>> {
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
