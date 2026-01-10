use crate::bin_path::BinPath;
use crate::editor::Editor;
use crate::parser::{Command, Parser};
use crate::pipeline::Pipeline;
use crate::print;
use std::cell::RefCell;
use std::fmt::Display;
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

    fn eval(&mut self) -> anyhow::Result<()> {
        if self.command.args.is_empty() {
            return Ok(());
        }

        Pipeline::new(&self.command, self.bin_path.borrow_mut()).run()?;
        Ok(())
    }

    pub fn repl(&mut self) -> anyhow::Result<()> {
        loop {
            print_err(self.read());
            print_err(self.eval());
        }
    }
}

fn print_err<T, E: Display>(result: Result<T, E>) {
    match result {
        Ok(_) => {}
        Err(err) => print!("{}\n", err),
    }
}
