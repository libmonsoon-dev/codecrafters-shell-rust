use crate::bin_path::BinPath;
use crate::editor::Editor;
use crate::parser::{Command, OutputStream};
use crate::{print_to, BUILTIN_COMMANDS};
use anyhow::{bail, Context};
use rustyline::history::History;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::Write;
use std::rc::Rc;
use std::{env, fs, io, mem, process, thread};

pub struct Pipeline<'a> {
    cmd: &'a Command,
    bin_path: Rc<RefCell<BinPath>>,
    editor: Rc<RefCell<Editor>>,
    threads: Vec<thread::JoinHandle<()>>,
}

impl<'a> Pipeline<'a> {
    pub fn new(
        cmd: &'a Command,
        bin_path: Rc<RefCell<BinPath>>,
        editor: Rc<RefCell<Editor>>,
    ) -> Self {
        Self {
            cmd,
            bin_path,
            editor,
            threads: Vec::with_capacity(4),
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut command = self.cmd;
        let mut process = self.call(&self.cmd.args, None)?;

        while let Some(output) = command.output() {
            let OutputStream::Pipe(pipe) = &output.to else {
                break;
            };

            let next_process = self.call(&pipe.args, Some(process.stdout()))?;
            process.wait(&mut self.threads);

            command = pipe;
            process = next_process;
        }

        self.copy_stdout(process.stdout(), command.get_output()?);
        self.copy_stderr(process.stderr(), command.get_error_output()?);
        process.wait(&mut self.threads);

        for thread in self.threads.drain(..) {
            thread.join().unwrap();
        }

        Ok(())
    }

    fn call(
        &mut self,
        args: &'a Vec<String>,
        stdin: Option<ProcessStdout>,
    ) -> anyhow::Result<Box<dyn Process + 'a>> {
        if BUILTIN_COMMANDS.contains(&&*args[0]) {
            return Ok(Box::new(BuiltinProcess::new(
                args,
                Rc::clone(&self.bin_path),
                Rc::clone(&self.editor),
            )));
        }

        if let Some(_) = self.bin_path.borrow_mut().lookup(&args[0])? {
            return Ok(Box::new(ExternalProcess::new(args, stdin)));
        }

        bail!("{}: command not found", args[0]);
    }

    fn copy_stdout<T: io::Write + Send + 'static>(&mut self, stdout: ProcessStdout, mut output: T) {
        let mut stdout: Box<dyn io::Read + Send + 'static> = match stdout {
            ProcessStdout::ChildStdout(stdout) => Box::new(stdout),
            ProcessStdout::Buffer(buf) => Box::new(io::Cursor::new(buf)),
        };

        let stdout_thread = thread::spawn(move || {
            io::copy(&mut stdout, &mut output).unwrap();
        });
        self.threads.push(stdout_thread);
    }

    fn copy_stderr<T: io::Write + Send + 'static>(&mut self, stderr: ProcessStderr, mut output: T) {
        let mut stderr: Box<dyn io::Read + Send + 'static> = match stderr {
            ProcessStderr::ChildStderr(stderr) => Box::new(stderr),
            ProcessStderr::Buffer(buf) => Box::new(io::Cursor::new(buf)),
        };

        let stderr_thread = thread::spawn(move || {
            io::copy(&mut stderr, &mut output).unwrap();
        });
        self.threads.push(stderr_thread);
    }
}

trait Process {
    fn stdout(&mut self) -> ProcessStdout;

    fn stderr(&mut self) -> ProcessStderr;

    fn wait(&mut self, threads: &mut Vec<thread::JoinHandle<()>>);
}

enum ProcessStdout {
    ChildStdout(process::ChildStdout),
    Buffer(Vec<u8>),
}

enum ProcessStderr {
    ChildStderr(process::ChildStderr),
    Buffer(Vec<u8>),
}

struct BuiltinProcess<'a> {
    args: &'a Vec<String>,
    bin_path: Rc<RefCell<BinPath>>,
    editor: Rc<RefCell<Editor>>,
    output: Vec<u8>,
}

impl<'a> BuiltinProcess<'a> {
    fn new(
        args: &'a Vec<String>,
        bin_path: Rc<RefCell<BinPath>>,
        editor: Rc<RefCell<Editor>>,
    ) -> Self {
        let mut p = Self {
            args,
            bin_path,
            editor,
            output: Vec::new(),
        };

        match p.args[0].as_ref() {
            "exit" => process::exit(0),
            "echo" => p.echo_builtin().unwrap(),
            "type" => p.type_builtin().unwrap(),
            "pwd" => print_to!(p.output, "{}\n", env::current_dir().unwrap().display()),
            "cd" => p.cd_builtin().unwrap(),
            "history" => p.history_builtin().unwrap(),
            _ => unimplemented!("builtin command {}", p.args[0]),
        }

        p
    }

    fn type_builtin(&mut self) -> io::Result<()> {
        let _ = self.args.clone()[1..]
            .iter()
            .try_for_each(|arg| -> io::Result<()> {
                if BUILTIN_COMMANDS.contains(&arg.as_str()) {
                    print_to!(self.output, "{} is a shell builtin\n", arg);
                    return Ok(());
                }

                if let Some(path) = self.bin_path.borrow_mut().lookup(&arg)? {
                    print_to!(self.output, "{} is {}\n", arg, path.display());
                    return Ok(());
                }

                print_to!(self.output, "{}: not found\n", arg);

                Ok(())
            })?;

        Ok(())
    }

    fn cd_builtin(&mut self) -> io::Result<()> {
        let path = if self.args.len() == 1 || self.args[1] == "~" {
            env::var("HOME").unwrap()
        } else {
            self.args[1].clone()
        };
        let attr = fs::metadata(&path);
        if matches!(attr, Err(ref err) if err.kind() == io::ErrorKind::NotFound) {
            print_to!(self.output, "cd: {path}: No such file or directory\n");
            return Ok(());
        }

        env::set_current_dir(&path)?;

        Ok(())
    }

    fn echo_builtin(&mut self) -> io::Result<()> {
        let str = self.args[1..].join(" ");
        print_to!(self.output, "{str}\n");

        Ok(())
    }

    fn history_builtin(&mut self) -> anyhow::Result<()> {
        let mut editor = self.editor.borrow_mut();

        if self.args.len() >= 3 && self.args[1] == "-r" {
            editor.history_mut().load((self.args[2]).as_ref())?
        } else if self.args.len() >= 3 && self.args[1] == "-w" {
            editor.history_mut().save((self.args[2]).as_ref())?
        } else if self.args.len() >= 3 && self.args[1] == "-a" {
            editor.history_mut().append((self.args[2]).as_ref())?
        } else if self.args.len() >= 2 {
            let num: usize = self.args[1].parse().context("failed to parse number")?;
            let iter = editor.history().iter().enumerate();

            last_n(iter, num)
                .into_iter()
                .for_each(|(num, line)| print_to!(self.output, "\t{num}  {line}\n"));
        } else {
            let iter = editor.history().iter().enumerate();
            iter.for_each(|(num, line)| print_to!(self.output, "\t{num}  {line}\n"))
        };

        Ok(())
    }
}

fn last_n<T>(iter: impl Iterator<Item = T>, n: usize) -> VecDeque<T> {
    let mut buffer = VecDeque::with_capacity(n);

    for item in iter {
        if buffer.len() == n {
            buffer.pop_front();
        }

        buffer.push_back(item);
    }

    buffer
}

impl<'a> Process for BuiltinProcess<'a> {
    fn stdout(&mut self) -> ProcessStdout {
        ProcessStdout::Buffer(mem::take(&mut self.output))
    }

    fn stderr(&mut self) -> ProcessStderr {
        ProcessStderr::Buffer(Vec::new())
    }

    fn wait(&mut self, _threads: &mut Vec<thread::JoinHandle<()>>) {
        // Noop
    }
}

struct ExternalProcess {
    stdin_buf: Option<Vec<u8>>,
    child: Option<process::Child>,
}

impl<'a> ExternalProcess {
    fn new(args: &'a Vec<String>, stdin: Option<ProcessStdout>) -> Self {
        let mut cmd = process::Command::new(&args[0]);

        args[1..].iter().for_each(|arg| {
            cmd.arg(arg);
        });

        let mut stdin_buf = None;
        let stdin = stdin
            .and_then(|stdin| match stdin {
                ProcessStdout::ChildStdout(child) => Some(process::Stdio::from(child)),
                ProcessStdout::Buffer(buf) => {
                    stdin_buf = Some(buf);
                    None
                }
            })
            .unwrap_or(process::Stdio::piped());

        let child = cmd
            .stdin(stdin)
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped())
            .spawn()
            .unwrap();

        Self {
            stdin_buf,
            child: Some(child),
        }
    }
}

impl Process for ExternalProcess {
    fn stdout(&mut self) -> ProcessStdout {
        ProcessStdout::ChildStdout(
            self.child
                .as_mut()
                .unwrap()
                .stdout
                .take()
                .expect("handle present"),
        )
    }

    fn stderr(&mut self) -> ProcessStderr {
        ProcessStderr::ChildStderr(
            self.child
                .as_mut()
                .unwrap()
                .stderr
                .take()
                .expect("handle present"),
        )
    }

    fn wait(&mut self, threads: &mut Vec<thread::JoinHandle<()>>) {
        let mut child = mem::take(&mut self.child).unwrap();

        match self.stdin_buf {
            Some(ref mut buf) => child
                .stdin
                .take()
                .expect("handle present")
                .write_all(buf)
                .unwrap(),
            None => {}
        }

        let process = thread::spawn(move || {
            child.wait().unwrap();
        });

        threads.push(process);
    }
}
