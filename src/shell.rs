use crate::bin_path::BinPath;
use crate::editor::Editor;
use crate::parser::{Command, OutputStream, Parser};
use crate::{print, BUILTIN_COMMANDS};
use duct::cmd;
use std::cell::RefCell;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;
use std::rc::Rc;

pub struct Shell {
    editor: Editor,
    bin_path: Rc<RefCell<BinPath>>,
    input_buffer: String,
    command: Command,
}

impl Shell {
    pub fn new() -> anyhow::Result<Shell> {
        let bin_path = Rc::new(RefCell::new(BinPath::new()));

        Ok(Shell {
            editor: Editor::new(bin_path.clone())?,
            bin_path,
            input_buffer: String::new(),
            command: Command {
                args: Vec::new(),
                redirects: Vec::new(),
            },
        })
    }

    fn read(&mut self) -> anyhow::Result<()> {
        self.input_buffer = self.editor.readline("$ ")?;

        //TODO: pass this vectors to parser to avoid allocations
        self.command = Parser::new(self.input_buffer.clone()).parse();

        Ok(())
    }

    fn eval(&mut self) -> io::Result<()> {
        if self.command.args.is_empty() {
            return Ok(());
        }

        if BUILTIN_COMMANDS.contains(&self.command.args[0].as_ref()) {
            match self.command.args[0].as_ref() {
                "exit" => process::exit(0),
                "echo" => self.echo_builtin()?,
                "type" => self.type_builtin()?,
                "pwd" => print!("{}\n", env::current_dir()?.display()),
                "cd" => self.cd_builtin()?,
                _ => unimplemented!("builtin command {}", self.command.args[0]),
            }

            return Ok(());
        }

        if let Some(bin) = self.bin_path.borrow_mut().lookup(&self.command.args[0])? {
            let mut command = &self.command;
            let mut expression = cmd(&bin, self.command.args[1..].iter());

            while let Some(output) = command.output() {
                let OutputStream::Pipe(pipe) = &output.to else {
                    break;
                };

                command = pipe;
                expression = expression.pipe(cmd(&pipe.args[0], pipe.args[1..].iter()));
            }

            let output = expression.stdout_capture().stderr_capture().run()?;
            command.get_output()?.write_all(&output.stdout)?;
            command.get_error_output()?.write_all(&output.stderr)?;

            return Ok(());
        }

        print!("{}: command not found\n", self.command.args[0].trim());
        Ok(())
    }

    pub fn repl(&mut self) -> anyhow::Result<()> {
        loop {
            self.read()?;
            self.eval()?;
        }
    }

    fn type_builtin(&mut self) -> io::Result<()> {
        let _ = &self.command.args.clone()[1..]
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
        let path = if self.command.args.len() == 1 || self.command.args[1] == "~" {
            env::var("HOME").unwrap()
        } else {
            self.command.args[1].clone()
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
        let str = self.command.args[1..].join(" ");
        self.command
            .get_output()?
            .write_fmt(format_args!("{str}\n"))?;
        self.command.get_error_output()?; //create file if needed

        Ok(())
    }
}
