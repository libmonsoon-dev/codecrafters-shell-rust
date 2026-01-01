use crate::escape_sequence_interpreter::Interpreter;
use crate::lexer::{Lexer, Token, TokenKind};

pub struct Parser {
    input: Vec<Token>,
    argument_buffer: String,
    position: usize,
    quotes: Vec<TokenKind>,
    echo: bool,
    arg_n: usize,
}

impl Parser {
    pub fn new(input: String) -> Self {
        Self {
            input: Lexer::new(input).lex(),
            argument_buffer: String::new(),
            position: 0,
            quotes: Vec::new(),
            echo: false,
            arg_n: 0,
        }
    }

    pub fn parse(&mut self) -> Vec<String> {
        let mut output: Vec<String> = Vec::new();

        while !self.is_eof() {
            if let Some(arg) = self.next_argument() {
                output.push(arg);
            }
        }

        output
    }

    fn is_eof(&self) -> bool {
        self.position >= self.input.len()
    }

    fn next_argument(&mut self) -> Option<String> {
        match self.current_token() {
            token if token.kind == TokenKind::SingleQuote => self.handle_single_quote(),
            token if token.kind == TokenKind::DoubleQuote => self.handle_double_quote(),
            token if token.kind == TokenKind::String => self.handle_string(),
            token if token.kind == TokenKind::Whitespace => self.handle_whitespace(),
            token if token.kind == TokenKind::EOF => self.handle_eof(),
            token => unimplemented!("{token:?} handling"),
        }
    }

    fn current_token(&self) -> &Token {
        &self.input[self.position]
    }

    fn handle_single_quote(&mut self) -> Option<String> {
        if !self.quotes.is_empty() && self.quotes.last().unwrap() == &TokenKind::SingleQuote {
            self.quotes.pop();
        } else if self.quotes.is_empty() {
            self.quotes.push(TokenKind::SingleQuote);
        } else {
            self.argument_buffer.push('\'')
        }
        self.position += 1;

        None
    }

    fn handle_double_quote(&mut self) -> Option<String> {
        if !self.quotes.is_empty() && self.quotes.last().unwrap() == &TokenKind::DoubleQuote {
            self.quotes.pop();
        } else {
            self.quotes.push(TokenKind::DoubleQuote);
        }
        self.position += 1;

        None
    }

    fn handle_string(&mut self) -> Option<String> {
        let mut lexeme = self.current_token().lexeme.clone();
        if self.echo {
            lexeme = self.unescape(lexeme);
        }
        self.argument_buffer.push_str(&lexeme);
        self.position += 1;

        None
    }

    fn unescape(&mut self, lexeme: String) -> String {
        if !lexeme.contains('\\') {
            return lexeme;
        }

        Interpreter::new(&lexeme, &mut self.quotes).interpret()
    }

    fn handle_whitespace(&mut self) -> Option<String> {
        let result = if !self.quotes.is_empty() {
            self.argument_buffer
                .push_str(&self.current_token().lexeme.clone());

            None
        } else {
            self.flush_buf()
        };

        self.position += 1;
        result
    }

    fn flush_buf(&mut self) -> Option<String> {
        if self.argument_buffer.is_empty() {
            return None;
        }

        let buf = self.argument_buffer.clone();
        self.argument_buffer.clear();

        if self.arg_n == 0 {
            self.echo = buf == "echo";
        }

        self.arg_n += 1;
        Some(buf)
    }

    fn handle_eof(&mut self) -> Option<String> {
        let result = self.flush_buf();
        self.position += 1;

        result
    }
}

#[cfg(test)]
mod tests {
    use super::Parser;
    use pretty_assertions::assert_eq;

    #[test]
    fn consecutive_spaces_are_collapsed_unless_quoted() {
        let mut parser = Parser::new(String::from(r#"hello    world"#));
        let args = parser.parse();
        assert_eq!(args, vec![String::from("hello"), String::from("world")]);
    }

    #[test]
    fn spaces_are_preserved_within_quotes() {
        let mut parser = Parser::new(String::from(r#"'hello    world'"#));
        let args = parser.parse();
        assert_eq!(args, vec![String::from("hello    world")]);
    }

    #[test]
    fn adjacent_quoted_strings_are_concatenated() {
        let mut parser = Parser::new(String::from(r#"'hello''world'"#));
        let args = parser.parse();
        assert_eq!(args, vec![String::from("helloworld")]);
    }

    #[test]
    fn empty_single_quotes_are_ignored() {
        let mut parser = Parser::new(String::from(r#"hello''world"#));
        let args = parser.parse();
        assert_eq!(args, vec![String::from("helloworld")]);
    }

    #[test]
    fn multiple_spaces_preserved() {
        let mut parser = Parser::new(String::from(r#""hello    world""#));
        let args = parser.parse();
        assert_eq!(args, vec![String::from("hello    world")]);
    }

    #[test]
    fn quoted_strings_next_to_each_other_are_concatenated() {
        let mut parser = Parser::new(String::from(r#""hello""world""#));
        let args = parser.parse();
        assert_eq!(args, vec![String::from("helloworld")]);
    }

    #[test]
    fn separate_arguments() {
        let mut parser = Parser::new(String::from(r#""hello" "world""#));
        let args = parser.parse();
        assert_eq!(args, vec![String::from("hello"), String::from("world")]);
    }

    #[test]
    fn single_quotes_inside_are_literal() {
        let mut parser = Parser::new(String::from(r#""shell's test""#));
        let args = parser.parse();
        assert_eq!(args, vec![String::from("shell's test")]);
    }

    #[test]
    fn each_backslash_creates_a_literal_space_as_part_of_one_argument() {
        let mut parser = Parser::new(String::from(r#"three\ \ \ spaces"#));
        let args = parser.parse();
        assert_eq!(args, vec![String::from(r#"three\ \ \ spaces"#)]);
    }

    #[test]
    fn the_backslash_preserves_the_first_space_literally_but_the_shell_collapses_the_subsequent_unescaped_spaces()
     {
        let mut parser = Parser::new(String::from(r#"before\     after"#));
        let args = parser.parse();
        assert_eq!(args, vec![String::from("before\\ "), String::from("after")]);
    }

    #[test]
    fn backslash_n_becomes_just_n() {
        let mut parser = Parser::new(String::from(r#"test\nexample"#));
        let args = parser.parse();
        assert_eq!(args, vec![String::from(r#"test\nexample"#)]);
    }

    #[test]
    fn the_first_backslash_escapes_the_second() {
        let mut parser = Parser::new(String::from(r#"hello\\world"#));
        let args = parser.parse();
        assert_eq!(args, vec![String::from(r#"hello\\world"#)]);
    }

    #[test]
    fn backslash_quote_makes_the_quote_literal_character() {
        let mut parser = Parser::new(String::from(r#"\'hello\'"#));
        let args = parser.parse();
        assert_eq!(args, vec![String::from(r#"\'hello\'"#)]);
    }

    #[test]
    fn echo_each_backslash_creates_a_literal_space_as_part_of_one_argument() {
        let mut parser = Parser::new(String::from(r#"echo three\ \ \ spaces"#));
        let args = parser.parse();
        assert_eq!(
            args,
            vec![String::from("echo"), String::from("three   spaces")]
        );
    }

    #[test]
    fn echo_the_backslash_preserves_the_first_space_literally_but_the_shell_collapses_the_subsequent_unescaped_spaces()
     {
        let mut parser = Parser::new(String::from(r#"echo before\  after"#));
        let args = parser.parse();
        assert_eq!(
            args,
            vec![
                String::from("echo"),
                String::from("before "),
                String::from("after")
            ]
        );
    }

    #[test]
    fn echo_backslash_n_becomes_just_n() {
        let mut parser = Parser::new(String::from(r#"echo test\nexample"#));
        let args = parser.parse();
        assert_eq!(
            args,
            vec![String::from("echo"), String::from("testnexample")]
        );
    }

    #[test]
    fn echo_first_backslash_escapes_the_second() {
        let mut parser = Parser::new(String::from(r#"echo hello\\world"#));
        let args = parser.parse();
        assert_eq!(
            args,
            vec![String::from("echo"), String::from(r#"hello\world"#)]
        );
    }

    #[test]
    fn echo_backslash_quote_makes_the_quote_literal_character() {
        let mut parser = Parser::new(String::from(r#"echo \'hello\'"#));
        let args = parser.parse();
        assert_eq!(args, vec![String::from("echo"), String::from("'hello'")]);
    }

    #[test]
    fn echo_backslashes_in_single_quotes() {
        let mut parser = Parser::new(String::from(r#"echo 'shell\\\nscript'"#));
        let args = parser.parse();
        assert_eq!(
            args,
            vec![String::from("echo"), String::from(r#"shell\\\nscript"#)]
        );
    }

    #[test]
    fn echo_backslashes_in_single_quotes_escape_double_quote() {
        let mut parser = Parser::new(String::from(r#"echo 'example\"test'"#));
        let args = parser.parse();
        assert_eq!(
            args,
            vec![String::from("echo"), String::from(r#"example\"test"#)]
        );
    }
}
