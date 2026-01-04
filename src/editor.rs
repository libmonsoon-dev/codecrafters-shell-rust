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
        let config = rustyline::Config::builder()
            .completion_type(rustyline::CompletionType::List)
            .indent_size(0)
            .build();

        let mut editor = rustyline::Editor::<Helper, DefaultHistory>::with_config(config)?;
        editor.set_helper(Some(Helper { bin_path }));

        Ok(Self { editor })
    }

    pub fn readline(&mut self, prompt: &str) -> rustyline::Result<String> {
        self.editor.readline(prompt)
    }
}
