use crate::lexer::{Lexer, Token, TokenKind};
use std::{fs, io, mem};

pub struct Parser {
    input: Vec<Token>,
    argument_buffer: String,
    position: usize,
    quotes: Vec<TokenKind>,
    args: Vec<String>,
    redirects: Vec<Redirect>,
}

impl Parser {
    pub fn new(input: String) -> Self {
        Self {
            input: Lexer::new(input).lex(),
            argument_buffer: String::new(),
            position: 0,
            quotes: Vec::with_capacity(1),
            args: Vec::new(),
            redirects: Vec::new(),
        }
    }

    pub fn parse(&mut self) -> (Vec<String>, Vec<Redirect>) {
        while !self.is_eof() {
            self.process_next_lexeme();
        }

        (mem::take(&mut self.args), mem::take(&mut self.redirects))
    }

    fn is_eof(&self) -> bool {
        self.position >= self.input.len()
    }

    fn process_next_lexeme(&mut self) {
        if let Some(str) = self.match_current_token() {
            self.args.push(str);
        }

        self.position += 1;
    }

    fn match_current_token(&mut self) -> Option<String> {
        match self.current_token().kind {
            TokenKind::SingleQuote => self.handle_single_quote(),
            TokenKind::DoubleQuote => self.handle_double_quote(),
            TokenKind::String => self.handle_string(),
            TokenKind::EscapeSequence => self.handle_escape_sequence(),
            TokenKind::Whitespace => self.handle_whitespace(),
            TokenKind::EOF => self.handle_eof(),
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

        None
    }

    fn handle_double_quote(&mut self) -> Option<String> {
        if self.quotes.is_empty() {
            self.quotes.push(TokenKind::DoubleQuote);
            return None;
        }

        if self.quotes.last().unwrap() == &TokenKind::DoubleQuote {
            self.quotes.pop();
        } else if self.quotes.last().unwrap() == &TokenKind::SingleQuote {
            self.argument_buffer.push('"');
        } else {
            unimplemented!(
                "handle double quote if current quotes are {:?}",
                self.quotes
            );
        }

        None
    }

    fn handle_string(&mut self) -> Option<String> {
        let lexeme = self.current_token().lexeme.clone();
        if lexeme.contains('>') {
            self.handle_redirect()
        } else {
            self.argument_buffer.push_str(&lexeme)
        }

        None
    }

    //TODO: return errors
    fn handle_redirect(&mut self) {
        let token = self.current_token();
        let mut chars = token.lexeme.chars().peekable();

        let mut from = OutputStream::default();
        match chars.peek().unwrap() {
            '1' => {
                from = OutputStream::Stdout;
                chars.next();
            }
            '2' => {
                from = OutputStream::Stderr;
                chars.next();
            }
            _ => {}
        }

        let next = chars.next().unwrap();
        let redirect_type: RedirectType;
        if next == '>' {
            redirect_type = if chars.peek() != None && chars.peek().unwrap() == &'>' {
                chars.next();
                RedirectType::Append
            } else {
                RedirectType::Overwrite
            }
        } else {
            panic!("unexpected char '{next:?}' while waiting for '>'")
        }

        let remaining = chars.collect::<String>();
        let to = if remaining.len() == 0 {
            self.position += 1;
            self.next_string()
        } else {
            self.argument_buffer.push_str(&remaining);
            self.position += 1;
            self.next_string()
        };

        self.redirects.push(Redirect {
            from,
            redirect_type,
            to,
        })
    }

    fn next_string(&mut self) -> String {
        while !self.is_eof() {
            if let Some(str) = self.match_current_token() {
                return str;
            }

            self.position += 1;
        }

        panic!("unexpected EOF")
    }

    fn handle_escape_sequence(&mut self) -> Option<String> {
        let lexeme = self.current_token().lexeme.clone();
        let escape_char = lexeme.chars().nth(1).unwrap();

        if self.quotes.is_empty() {
            self.argument_buffer.push(escape_char);
        } else if self.quotes.last() == Some(&TokenKind::DoubleQuote) {
            static DOUBLE_QUOTE_ESCAPABLE: &[char] = &['"', '\\', '$', '`', '\n'];
            if DOUBLE_QUOTE_ESCAPABLE.contains(&escape_char) {
                self.argument_buffer.push(escape_char);
            } else {
                self.argument_buffer.push('\\');
                self.argument_buffer.push(escape_char);
            }
        } else if self.quotes.last() == Some(&TokenKind::SingleQuote) {
            self.argument_buffer.push('\\');
            self.argument_buffer.push(escape_char);
        } else {
            unimplemented!(
                "handle escape sequence if current quotes are {:?}",
                self.quotes
            );
        }

        None
    }

    fn handle_whitespace(&mut self) -> Option<String> {
        if !self.quotes.is_empty() {
            self.argument_buffer
                .push_str(&self.current_token().lexeme.clone());

            None
        } else {
            self.flush_buf()
        }
    }

    fn handle_eof(&mut self) -> Option<String> {
        self.flush_buf()
    }

    fn flush_buf(&mut self) -> Option<String> {
        if self.argument_buffer.is_empty() {
            return None;
        }

        Some(mem::take(&mut self.argument_buffer))
    }
}

#[derive(Default, PartialEq, Debug)]
pub enum OutputStream {
    #[default]
    Stdout,
    Stderr,
}

#[derive(PartialEq, Debug)]
pub enum RedirectType {
    Overwrite,
    Append,
}

#[derive(PartialEq, Debug)]
pub struct Redirect {
    pub from: OutputStream,
    pub redirect_type: RedirectType,
    pub to: String,
}

impl Redirect {
    pub fn open_output(&self) -> io::Result<fs::File> {
        Ok(match self.redirect_type {
            RedirectType::Overwrite => fs::File::create(&self.to)?,
            RedirectType::Append => fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(&self.to)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case(r#"hello    world"#, vec!["hello", "world"], vec![])]
    #[case(r#"'hello    world'"#, vec!["hello    world"], vec![])]
    #[case(r#"'hello''world'"#, vec!["helloworld"], vec![])]
    #[case(r#"hello''world"#, vec!["helloworld"], vec![])]
    #[case(r#""hello    world""#, vec!["hello    world"], vec![])]
    #[case(r#""hello""world""#, vec!["helloworld"], vec![])]
    #[case(r#""hello" "world""#, vec!["hello", "world"], vec![])]
    #[case(r#""shell's test""#, vec!["shell's test"], vec![])]
    #[case(r#"echo three\ \ \ spaces"#, vec!["echo", "three   spaces"], vec![])]
    #[case(r#"echo before\  after"#, vec!["echo", "before ", "after"], vec![])]
    #[case(r#"echo test\nexample"#, vec!["echo", "testnexample"], vec![])]
    #[case(r#"echo hello\\world"#, vec!["echo", r#"hello\world"#], vec![])]
    #[case(r#"echo \'hello\'"#, vec!["echo", "'hello'"], vec![])]
    #[case(r#"echo 'shell\\\nscript'"#, vec!["echo", r#"shell\\\nscript"#], vec![])]
    #[case(r#"echo 'example\"test'"#, vec!["echo", r#"example\"test"#], vec![])]
    #[case(r#"echo 'world\"testhello\"shell'"#, vec!["echo", r#"world\"testhello\"shell"#], vec![])]
    #[case(r#"echo "hello'test'\\'script""#, vec!["echo", r#"hello'test'\'script"#], vec![])]
    #[case(r#"cat "/tmp/fox/\"f 32\"""#, vec!["cat", r#"/tmp/fox/"f 32""#], vec![])]
    #[case(r#"cat "/tmp/fox/\"f\\87\"""#, vec!["cat", r#"/tmp/fox/"f\87""#], vec![])]
    #[case(r#"cat "/tmp/fox/f17""#, vec!["cat", "/tmp/fox/f17"], vec![])]
    #[case(r#"'my program' argument1"#, vec!["my program", "argument1"], vec![])]
    #[case(r#""exe with spaces" file.txt"#, vec!["exe with spaces", "file.txt"], vec![])]
    #[case(r#"'exe with "quotes"' file"#, vec![r#"exe with "quotes""#, "file"], vec![])]
    #[case(r#""exe with 'single quotes'" file"#, vec!["exe with 'single quotes'", "file"], vec![])]
    #[case(r#"'exe with \n newline' arg"#, vec![r#"exe with \n newline"#, "arg"], vec![])]
    #[case("echo hello > output.txt", vec!["echo", "hello"], vec![Redirect{
        from: OutputStream::default(),
        redirect_type: RedirectType::Overwrite,
        to: String::from("output.txt"),
    }])]
    #[case("echo hello 1> file\\ txt", vec!["echo", "hello"], vec![Redirect{
        from: OutputStream::Stdout,
        redirect_type: RedirectType::Overwrite,
        to: String::from("file txt"),
    }])]
    #[case("echo hello 1>fi''le.txt", vec!["echo", "hello"], vec![Redirect{
        from: OutputStream::Stdout,
        redirect_type: RedirectType::Overwrite,
        to: String::from("file.txt"),
    }])]
    fn parser_test(
        #[case] input: &str,
        #[case] expected: Vec<&str>,
        #[case] expected_redirects: Vec<Redirect>,
    ) {
        let mut parser = Parser::new(String::from(input));
        let (args, redirects) = parser.parse();
        assert_eq!(
            args,
            expected
                .iter()
                .cloned()
                .map(String::from)
                .collect::<Vec<String>>()
        );

        assert_eq!(redirects, expected_redirects);
    }
}
