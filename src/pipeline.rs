use crate::bin_path::BinPath;
use crate::parser::{Command, OutputStream};
use crate::BUILTIN_COMMANDS;
use anyhow::bail;
use std::cell::RefMut;
use std::{env, fs, io, mem, process, thread};

pub struct Pipeline<'a> {
    cmd: &'a Command,
    bin_path: RefMut<'a, BinPath>,
    threads: Vec<thread::JoinHandle<()>>,
}

impl<'a> Pipeline<'a> {
    pub fn new(cmd: &'a Command, bin_path: RefMut<'a, BinPath>) -> Self {
        Self {
            cmd,
            bin_path,
            threads: Vec::with_capacity(4),
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut command = self.cmd;
        let mut process = self.call(&self.cmd.args, None)?;
        let mut piped = false;

        while let Some(output) = command.output() {
            let OutputStream::Pipe(pipe) = &output.to else {
                break;
            };

            piped = true;
            let next_process = self.call(&pipe.args, Some(process.stdout()))?;
            process.wait(&mut self.threads);

            command = pipe;
            process = next_process;
        }

        if !piped {
            self.copy_stdin(process.stdin(), io::empty())
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
            //TODO:
            // match cmd.args[0].as_ref() {
            //     "exit" => process::exit(0),
            //     "echo" => self.echo_builtin(cmd)?,
            //     "type" => self.type_builtin(cmd)?,
            //     "pwd" => print!("{}\n", env::current_dir()?.display()),
            //     "cd" => self.cd_builtin(cmd)?,
            //     _ => unimplemented!("builtin command {}", cmd.args[0]),
            // }

            return Ok(Box::new(BuiltinProcess::new(args, stdin)));
        }

        if let Some(_) = self.bin_path.lookup(&args[0])? {
            return Ok(Box::new(ExternalProcess::new(args, stdin)));
        }

        bail!("{}: command not found", args[0].trim());
    }

    fn copy_stdin<T: io::Read + Send + 'static>(&mut self, stdin: ProcessStdin, mut input: T) {
        let mut stdin: Box<dyn io::Write + Send + 'static> = match stdin {
            ProcessStdin::ChildStdin(stdin) => Box::new(stdin),
        };

        let stdin_thread = thread::spawn(move || {
            io::copy(&mut input, &mut stdin).unwrap();
        });
        self.threads.push(stdin_thread);
    }

    fn copy_stdout<T: io::Write + Send + 'static>(&mut self, stdout: ProcessStdout, mut output: T) {
        let mut stdout: Box<dyn io::Read + Send + 'static> = match stdout {
            ProcessStdout::ChildStdout(stdout) => Box::new(stdout),
        };

        let stdout_thread = thread::spawn(move || {
            io::copy(&mut stdout, &mut output).unwrap();
        });
        self.threads.push(stdout_thread);
    }

    fn copy_stderr<T: io::Write + Send + 'static>(&mut self, stderr: ProcessStderr, mut output: T) {
        let mut stderr: Box<dyn io::Read + Send + 'static> = match stderr {
            ProcessStderr::ChildStderr(stderr) => Box::new(stderr),
        };

        let stderr_thread = thread::spawn(move || {
            io::copy(&mut stderr, &mut output).unwrap();
        });
        self.threads.push(stderr_thread);
    }

    fn type_builtin(&mut self, cmd: &Command) -> io::Result<()> {
        let _ = cmd.args.clone()[1..]
            .iter()
            .try_for_each(|arg| -> io::Result<()> {
                if BUILTIN_COMMANDS.contains(&arg.as_str()) {
                    print!("{} is a shell builtin\n", arg);
                    return Ok(());
                }

                if let Some(path) = self.bin_path.lookup(&arg)? {
                    print!("{} is {}\n", arg, path.display());
                    return Ok(());
                }

                print!("{}: not found\n", arg);

                Ok(())
            })?;

        Ok(())
    }

    fn cd_builtin(&mut self, cmd: &Command) -> io::Result<()> {
        let path = if cmd.args.len() == 1 || cmd.args[1] == "~" {
            env::var("HOME").unwrap()
        } else {
            cmd.args[1].clone()
        };
        let attr = fs::metadata(path.clone());
        if matches!(attr, Err(ref err) if err.kind() == io::ErrorKind::NotFound) {
            print!("cd: {path}: No such file or directory\n");
            return Ok(());
        }

        env::set_current_dir(path)?;

        Ok(())
    }

    fn echo_builtin(&mut self, cmd: &Command) -> io::Result<()> {
        let str = cmd.args[1..].join(" ");
        cmd.get_output()?.write_fmt(format_args!("{str}\n"))?;
        cmd.get_error_output()?; //create file if needed

        Ok(())
    }
}

trait Process {
    fn stdin(&mut self) -> ProcessStdin;

    fn stdout(&mut self) -> ProcessStdout;

    fn stderr(&mut self) -> ProcessStderr;

    fn wait(&mut self, threads: &mut Vec<thread::JoinHandle<()>>);
}

enum ProcessStdout {
    ChildStdout(process::ChildStdout),
    //TODO:
    // Buffer(String)
}

enum ProcessStdin {
    ChildStdin(process::ChildStdin),
}

enum ProcessStderr {
    ChildStderr(process::ChildStderr),
}

struct BuiltinProcess<'a> {
    args: &'a Vec<String>,
}

impl<'a> BuiltinProcess<'a> {
    fn new(args: &'a Vec<String>, stdin: Option<ProcessStdout>) -> Self {
        Self { args }
    }
}

impl<'a> Process for BuiltinProcess<'a> {
    fn stdin(&mut self) -> ProcessStdin {
        todo!()
    }

    fn stdout(&mut self) -> ProcessStdout {
        todo!()
    }

    fn stderr(&mut self) -> ProcessStderr {
        todo!()
    }

    fn wait(&mut self, threads: &mut Vec<thread::JoinHandle<()>>) {
        todo!()
    }
}

struct ExternalProcess {
    child: Option<process::Child>,
}

impl<'a> ExternalProcess {
    fn new(args: &'a Vec<String>, stdin: Option<ProcessStdout>) -> Self {
        let mut cmd = process::Command::new(&args[0]);

        args[1..].iter().for_each(|arg| {
            cmd.arg(arg);
        });

        let stdin = stdin
            .map(|stdin| match stdin {
                ProcessStdout::ChildStdout(child) => process::Stdio::from(child),
            })
            .unwrap_or(process::Stdio::piped());

        let child = cmd
            .stdin(stdin)
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped())
            .spawn()
            .unwrap();

        Self { child: Some(child) }
    }
}

impl Process for ExternalProcess {
    fn stdin(&mut self) -> ProcessStdin {
        ProcessStdin::ChildStdin(
            self.child
                .as_mut()
                .unwrap()
                .stdin
                .take()
                .expect("handle present"),
        )
    }

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

        let process = thread::spawn(move || {
            child.wait().unwrap();
        });

        threads.push(process);
    }
}
