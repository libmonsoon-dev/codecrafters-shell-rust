use rustyline::history::DefaultHistory;

pub struct Helper;

impl rustyline::hint::Hinter for Helper {
    type Hint = String;
}

impl rustyline::highlight::Highlighter for Helper {}
impl rustyline::validate::Validator for Helper {}

impl rustyline::Helper for Helper {}

pub type Editor = rustyline::Editor<Helper, DefaultHistory>;

pub fn new_read_line() -> anyhow::Result<Editor> {
    let mut editor = rustyline::Editor::new()?;

    editor.set_helper(Some(Helper));

    Ok(editor)
}
