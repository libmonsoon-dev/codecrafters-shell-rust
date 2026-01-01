use crate::lexer::TokenKind;
use std::mem;
use std::str::Chars;

pub struct Interpreter<'a> {
    input: &'a str,
    chars: Chars<'a>,
    quotes: &'a mut Vec<TokenKind>,
    output: String,
}

impl<'a> Interpreter<'a> {
    pub fn new(input: &'a str, quotes: &'a mut Vec<TokenKind>) -> Self {
        Self {
            input,
            chars: input.chars(),
            quotes,
            output: String::with_capacity(input.len()),
        }
    }

    pub fn interpret(&mut self) -> String {
        while let Some(c) = self.chars.next() {
            match c {
                '\'' => self.handle_single_quote(),
                '"' => self.handle_double_quote(),
                '\\' => self.handle_slash(),
                c => self.handle_char(c),
            }
        }

        mem::take(&mut self.output)
    }

    fn handle_single_quote(&mut self) {
        if self.quotes.is_empty() {
            self.quotes.push(TokenKind::SingleQuote);
        } else if self.quotes.last() == Some(&TokenKind::SingleQuote) {
            self.quotes.pop();
        } else {
            unimplemented!(
                "handle single quote if current quotes are {:?}",
                self.quotes
            );
        }
    }

    fn handle_double_quote(&mut self) {
        if self.quotes.is_empty() {
            self.quotes.push(TokenKind::DoubleQuote);
        } else if self.quotes.last() == Some(&TokenKind::DoubleQuote) {
            self.quotes.pop();
        } else {
            self.handle_char('"')
        }
    }

    fn handle_slash(&mut self) {
        if self.quotes.is_empty() {
            let c = self.chars.next().unwrap();
            self.handle_char(c)
        } else {
            self.handle_char('\\')
        }
    }

    fn handle_char(&mut self, c: char) {
        self.output.push(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn no_slash() {
        let input = String::from("hello");
        let mut quotes = Vec::new();
        let mut interpreter = Interpreter::new(&input, &mut quotes);
        assert_eq!(interpreter.interpret(), String::from("hello"));
    }

    #[test]
    fn each_backslash_creates_a_literal_space_as_part_of_one_argument() {
        let input = String::from(r#"three\ \ \ spaces"#);
        let mut quotes = Vec::new();
        let mut interpreter = Interpreter::new(&input, &mut quotes);
        assert_eq!(interpreter.interpret(), String::from("three   spaces"));
    }

    #[test]
    fn the_backslash_preserves_the_first_space_literally_but_the_shell_collapses_the_subsequent_unescaped_spaces()
     {
        let input = String::from(r#"before\ "#);
        let mut quotes = Vec::new();
        let mut interpreter = Interpreter::new(&input, &mut quotes);
        assert_eq!(interpreter.interpret(), String::from("before "));
    }

    #[test]
    fn backslash_n_becomes_just_n() {
        let input = String::from(r#"test\nexample"#);
        let mut quotes = Vec::new();
        let mut interpreter = Interpreter::new(&input, &mut quotes);
        assert_eq!(interpreter.interpret(), String::from("testnexample"));
    }

    #[test]
    fn the_first_backslash_escapes_the_second() {
        let input = String::from(r#"hello\\world"#);
        let mut quotes = Vec::new();
        let mut interpreter = Interpreter::new(&input, &mut quotes);
        assert_eq!(interpreter.interpret(), String::from(r#"hello\world"#));
    }

    #[test]
    fn backslash_quote_makes_the_quote_literal_character() {
        let input = String::from(r#"\'hello\'"#);
        let mut quotes = Vec::new();
        let mut interpreter = Interpreter::new(&input, &mut quotes);
        assert_eq!(interpreter.interpret(), String::from("'hello'"));
    }

    #[test]
    fn backslashes_in_single_quotes() {
        let input = String::from(r#"'shell\\\nscript'"#);
        let mut quotes = Vec::new();
        let mut interpreter = Interpreter::new(&input, &mut quotes);
        let res = interpreter.interpret();
        assert_eq!(res, String::from(r#"shell\\\nscript"#));
    }

    #[test]
    fn backslashes_in_single_quotes_escape_double_quote() {
        let input = String::from(r#"'example\"test'"#);
        let mut quotes = Vec::new();
        let mut interpreter = Interpreter::new(&input, &mut quotes);
        assert_eq!(interpreter.interpret(), String::from(r#"example\"test"#));
    }
}
