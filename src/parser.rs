use crate::interpreter::Interpreter;
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
    use rstest::rstest;

    #[rstest]
    #[case(r#"hello    world"#, vec!["hello", "world"])]
    #[case(r#"'hello    world'"#, vec!["hello    world"])]
    #[case(r#"'hello''world'"#, vec!["helloworld"])]
    #[case(r#"hello''world"#, vec!["helloworld"])]
    #[case(r#""hello    world""#, vec!["hello    world"])]
    #[case(r#""hello""world""#, vec!["helloworld"])]
    #[case(r#""hello" "world""#, vec!["hello", "world"])]
    #[case(r#""shell's test""#, vec!["shell's test"])]
    #[case(r#"three\ \ \ spaces"#, vec![r#"three\ \ \ spaces"#])]
    #[case(r#"before\     after"#, vec!["before\\ ", "after"])]
    #[case(r#"test\nexample"#, vec![r#"test\nexample"#])]
    #[case(r#"hello\\world"#, vec![r#"hello\\world"#])]
    #[case(r#"\'hello\'"#, vec![r#"\'hello\'"#])]
    #[case(r#"echo three\ \ \ spaces"#, vec!["echo", "three   spaces"])]
    #[case(r#"echo before\  after"#, vec!["echo", "before ", "after"])]
    #[case(r#"echo test\nexample"#, vec!["echo", "testnexample"])]
    #[case(r#"echo hello\\world"#, vec!["echo", r#"hello\world"#])]
    #[case(r#"echo \'hello\'"#, vec!["echo", "'hello'"])]
    #[case(r#"echo 'shell\\\nscript'"#, vec!["echo", r#"shell\\\nscript"#])]
    #[case(r#"echo 'example\"test'"#, vec!["echo", r#"example\"test"#])]
    #[case(r#"echo "hello'test'\\'script""#, vec!["echo", r#"hello'test'\'script"#])]
    fn parser_test(#[case] input: &str, #[case] expected: Vec<&str>) {
        let mut parser = Parser::new(String::from(input));
        let args = parser.parse();
        assert_eq!(
            args,
            expected
                .iter()
                .cloned()
                .map(String::from)
                .collect::<Vec<String>>()
        );
    }
}
