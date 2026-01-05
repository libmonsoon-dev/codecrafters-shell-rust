use crate::editor::Helper;
use crate::BUILTIN_COMMANDS;
use indexmap::IndexSet;
use rustyline::completion;
use std::path;

impl completion::Completer for Helper {
    type Candidate = completion::Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let (start, word) = completion::extract_word(line, pos, None, |c| c == ' ');
        let mut candidates = IndexSet::new();

        for comp in BUILTIN_COMMANDS {
            if comp.starts_with(word) {
                candidates.insert(new_pair(comp.to_string()));
            }
        }

        let mut bin_path = self.bin_path.borrow_mut();
        for bin in bin_path.bins() {
            let bin_path = bin.unwrap().display().to_string();

            if let Some(basename) = path::Path::new(&bin_path).file_name() {
                let basename = basename.display().to_string();
                if basename.starts_with(word) {
                    candidates.insert(new_pair(basename));
                }
            }
        }

        Ok((start, candidates.into_iter().collect()))
    }
}

fn new_pair(display: String) -> completion::Pair {
    completion::Pair {
        replacement: append_trailing_space(&display),
        display,
    }
}

fn append_trailing_space(path: &str) -> String {
    let mut result = String::with_capacity(path.len() + 1);
    result.push_str(path);
    result.push(' ');

    result
}
