use crate::bin_path::BinPath;
use rustyline::history::DefaultHistory;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Helper {
    pub(crate) bin_path: Rc<RefCell<BinPath>>,
}

impl rustyline::hint::Hinter for Helper {
    type Hint = String;
}

impl rustyline::highlight::Highlighter for Helper {}
impl rustyline::validate::Validator for Helper {}

impl rustyline::Helper for Helper {}

pub struct Editor {
    editor: rustyline::Editor<Helper, DefaultHistory>,
}

impl Editor {
    pub fn new(bin_path: Rc<RefCell<BinPath>>) -> anyhow::Result<Self> {
        let mut editor = rustyline::Editor::<Helper, DefaultHistory>::new()?;
        editor.set_helper(Some(Helper { bin_path }));

        Ok(Self { editor })
    }

    pub fn readline(&mut self, prompt: &str) -> rustyline::Result<String> {
        self.editor.readline(prompt)
    }
}
