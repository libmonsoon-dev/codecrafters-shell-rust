use crate::bin_path::BinPath;
use crate::editor::Editor;
use crate::parser::{Command, Parser};
use crate::pipeline::Pipeline;
use crate::{print, ExitError};
use std::cell::RefCell;
use std::env;
use std::env::VarError;
use std::rc::Rc;

pub struct Shell {
    editor: Rc<RefCell<Editor>>,
    bin_path: Rc<RefCell<BinPath>>,
    input_buffer: String,
    command: Command,
}

impl Shell {
    pub fn new() -> anyhow::Result<Shell> {
        let bin_path = Rc::new(RefCell::new(BinPath::new()));

        let shell = Shell {
            editor: Rc::new(RefCell::new(Editor::new(bin_path.clone())?)),
            bin_path,
            input_buffer: String::new(),
            command: Command {
                args: Vec::new(),
                redirects: Vec::new(),
            },
        };

        shell.read_history()?;
        Ok(shell)
    }

    fn read(&mut self) -> anyhow::Result<()> {
        self.input_buffer = self.editor.borrow_mut().readline("$ ")?;

        self.command = Parser::new(&self.input_buffer).parse();
        Ok(())
    }

    fn eval(&mut self) -> anyhow::Result<()> {
        if self.command.args.is_empty() {
            return Ok(());
        }

        self.new_pipeline(&self.command).run()?;
        Ok(())
    }

    fn new_pipeline<'a>(&'a self, command: &'a Command) -> Pipeline<'a> {
        Pipeline::new(command, Rc::clone(&self.bin_path), Rc::clone(&self.editor))
    }

    pub fn repl(&mut self) -> anyhow::Result<()> {
        loop {
            handle_err(self.read())?;
            handle_err(self.eval())?;
        }
    }

    fn read_history(&self) -> anyhow::Result<()> {
        let history_file = env::var("HISTFILE");
        match history_file {
            Err(VarError::NotPresent) => return Ok(()),
            _ => {}
        }

        let command = Command {
            args: vec![String::from("history"), String::from("-r"), history_file?],
            redirects: vec![],
        };
        self.new_pipeline(&command).run()?;

        Ok(())
    }

    fn append_history(&mut self) -> anyhow::Result<()> {
        let history_file = env::var("HISTFILE");
        match history_file {
            Err(VarError::NotPresent) => return Ok(()),
            _ => {}
        }

        let command = Command {
            args: vec![String::from("history"), String::from("-a"), history_file?],
            redirects: vec![],
        };
        self.new_pipeline(&command).run()?;

        Ok(())
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        self.append_history().unwrap();
    }
}

fn handle_err<T>(result: anyhow::Result<T>) -> anyhow::Result<()> {
    match result {
        Ok(_) => Ok(()),
        Err(err) if contain::<rustyline::error::ReadlineError>(err.chain()) => Err(err),
        Err(err) if contain::<ExitError>(err.chain()) => Err(err),
        Err(err) => {
            print!("{}\n", err);
            Ok(())
        }
    }
}

pub fn contain<T: std::error::Error + 'static>(chain: anyhow::Chain) -> bool {
    for cause in chain {
        if cause.downcast_ref::<T>().is_some() {
            return true;
        }
    }

    false
}
