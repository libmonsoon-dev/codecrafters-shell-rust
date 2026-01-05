use crate::editor::Helper;
use crate::BUILTIN_COMMANDS;
use indexmap::IndexSet;
use rustyline::completion;
use std::path;

impl completion::Completer for Helper {
    type Candidate = Pair;

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
                candidates.insert(Pair::new(comp.to_string()));
            }
        }

        let mut bin_path = self.bin_path.borrow_mut();
        for bin in bin_path.bins() {
            let bin_path = bin.unwrap().display().to_string();

            if let Some(basename) = path::Path::new(&bin_path).file_name() {
                let basename = basename.display().to_string();
                if basename.starts_with(word) {
                    candidates.insert(Pair::new(basename));
                }
            }
        }

        candidates.sort();

        Ok((start, candidates.into_iter().collect()))
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Pair {
    pub display: String,
    pub replacement: String,
}

impl Pair {
    fn new(display: String) -> Pair {
        Self {
            replacement: append_trailing_space(&display),
            display,
        }
    }
}

impl completion::Candidate for Pair {
    fn display(&self) -> &str {
        self.display.as_str()
    }

    fn replacement(&self) -> &str {
        self.replacement.as_str()
    }
}

fn append_trailing_space(path: &str) -> String {
    let mut result = String::with_capacity(path.len() + 1);
    result.push_str(path);
    result.push(' ');

    result
}
