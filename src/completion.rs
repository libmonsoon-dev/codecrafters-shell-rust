use crate::editor::Helper;
use crate::BUILTIN_COMMANDS;
use rustyline::completion::{extract_word, Completer};
use std::path;

impl Completer for Helper {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let (start, word) = extract_word(line, pos, None, |c| c == ' ');
        let mut candidates = Vec::new();

        for comp in BUILTIN_COMMANDS {
            if comp.starts_with(word) {
                candidates.push(append_trailing_space(comp));
            }
        }

        let mut bin_path = self.bin_path.borrow_mut();
        for bin in bin_path.bins() {
            let bin_path = bin.unwrap().display().to_string();

            if let Some(basename) = path::Path::new(&bin_path).file_name() {
                let basename = basename.display().to_string();
                if basename.starts_with(word) {
                    candidates.push(append_trailing_space(&basename))
                }
            }

            if bin_path.starts_with(word) {
                candidates.push(append_trailing_space(&bin_path));
            }
        }

        Ok((start, candidates))
    }
}

fn append_trailing_space(path: &str) -> String {
    let mut result = String::with_capacity(path.len() + 1);
    result.push_str(path);
    result.push(' ');

    result
}
