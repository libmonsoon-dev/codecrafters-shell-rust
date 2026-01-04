use crate::read_line::Helper;
use crate::BUILTIN_COMMANDS;
use rustyline::completion::{extract_word, Completer};

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
                let mut candidate = comp.to_string();
                candidate.push(' ');
                candidates.push(candidate);
            }
        }

        Ok((start, candidates))
    }
}
