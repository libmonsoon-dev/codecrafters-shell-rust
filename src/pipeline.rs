use crate::parser::{Command, OutputStream};
use std::thread::JoinHandle;
use std::{io, process, thread};

pub struct Pipeline<'a> {
    cmd: &'a Command,
    threads: Vec<JoinHandle<()>>,
}

impl<'a> Pipeline<'a> {
    pub fn new(cmd: &'a Command) -> Self {
        Self {
            cmd,
            threads: Vec::with_capacity(4),
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut command = self.cmd;
        let mut child = self.exec(&self.cmd.args, None)?;
        let mut piped = false;

        while let Some(output) = command.output() {
            let OutputStream::Pipe(pipe) = &output.to else {
                break;
            };

            piped = true;
            let next_child =
                self.exec(&pipe.args, child.stdout.take().map(process::Stdio::from))?;
            self.wait_process(child);

            command = pipe;
            child = next_child;
        }

        if !piped {
            let child_stdin = child.stdin.take().expect("handle present");
            self.copy_stdin(child_stdin, io::empty())
        }

        let child_stdout = child.stdout.take().expect("handle present");
        self.copy_stdout(child_stdout, command.get_output()?);

        let child_stderr = child.stderr.take().expect("handle present");
        self.copy_stderr(child_stderr, command.get_error_output()?);

        self.wait_process(child);

        for thread in self.threads.drain(..) {
            thread.join().unwrap();
        }

        Ok(())
    }

    fn exec(
        &mut self,
        args: &Vec<String>,
        stdin: Option<process::Stdio>,
    ) -> anyhow::Result<process::Child> {
        let mut cmd = process::Command::new(&args[0]);

        args[1..].iter().for_each(|arg| {
            cmd.arg(arg);
        });

        let child = cmd
            .stdin(stdin.unwrap_or(process::Stdio::piped()))
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped())
            .spawn()?;

        Ok(child)
    }

    fn copy_stdin<T: io::Read + Send + 'static>(
        &mut self,
        mut child_stdin: process::ChildStdin,
        mut input: T,
    ) {
        let stdin_thread = thread::spawn(move || {
            io::copy(&mut input, &mut child_stdin).unwrap();
        });
        self.threads.push(stdin_thread);
    }

    fn copy_stdout<T: io::Write + Send + 'static>(
        &mut self,
        mut child_stdout: process::ChildStdout,
        mut output: T,
    ) {
        let stdout_thread = thread::spawn(move || {
            io::copy(&mut child_stdout, &mut output).unwrap();
        });
        self.threads.push(stdout_thread);
    }

    fn copy_stderr<T: io::Write + Send + 'static>(
        &mut self,
        mut child_stderr: process::ChildStderr,
        mut output: T,
    ) {
        let stderr_thread = thread::spawn(move || {
            io::copy(&mut child_stderr, &mut output).unwrap();
        });
        self.threads.push(stderr_thread);
    }

    fn wait_process(&mut self, mut child: process::Child) {
        let process = thread::spawn(move || {
            child.wait().unwrap();
        });
        self.threads.push(process);
    }
}
