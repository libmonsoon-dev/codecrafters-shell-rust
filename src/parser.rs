use crate::lexer::{Lexer, Token, TokenKind};
use std::io::Write;
use std::{fs, io, mem};

pub struct Parser {
    input: Vec<Token>,
    argument_buffer: String,
    position: usize,
    quotes: Vec<TokenKind>,
    args: Vec<String>,
    redirects: Vec<Redirect>,
}

#[derive(PartialEq, Debug)]
pub struct Command {
    pub(crate) args: Vec<String>,
    pub(crate) redirects: Vec<Redirect>,
}

impl Command {
    pub fn new(args: Vec<&str>, redirects: Vec<Redirect>) -> Self {
        Self {
            args: args.into_iter().map(String::from).collect(),
            redirects,
        }
    }

    pub(crate) fn get_output(&self) -> io::Result<Box<dyn Write + Send>> {
        let Some(redirect) = self
            .redirects
            .iter()
            .find(|r| r.from == OutputStream::Stdout)
        else {
            return Ok(Box::new(io::stdout()));
        };

        let file = redirect.open_output()?;
        Ok(Box::new(file))
    }

    pub(crate) fn get_error_output(&mut self) -> io::Result<Box<dyn Write + Send>> {
        let Some(redirect) = self
            .redirects
            .iter()
            .find(|r| r.from == OutputStream::Stderr)
        else {
            return Ok(Box::new(io::stderr()));
        };

        let file = redirect.open_output()?;
        Ok(Box::new(file))
    }
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

    pub fn parse(&mut self) -> Command {
        while !self.is_eof() {
            self.process_next_lexeme();
        }

        self.current_command()
    }

    fn current_command(&mut self) -> Command {
        Command {
            args: mem::take(&mut self.args),
            redirects: mem::take(&mut self.redirects),
        }
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
        if lexeme == "|" {
            self.handle_pipe()
        } else if lexeme.contains('>') {
            self.handle_redirect()
        } else {
            self.argument_buffer.push_str(&lexeme)
        }

        None
    }

    fn handle_pipe(&mut self) {
        let args = mem::take(&mut self.args);
        let mut redirects = mem::take(&mut self.redirects);

        self.position += 1;
        while !self.is_eof() {
            //TODO: use iteration instead of recursion
            self.process_next_lexeme();
        }

        redirects.push(Redirect::new_pipe(self.current_command()));

        self.args = args;
        self.redirects = redirects;
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
        let to = OutputStream::File(if remaining.len() == 0 {
            self.position += 1;
            self.next_string()
        } else {
            self.argument_buffer.push_str(&remaining);
            self.position += 1;
            self.next_string()
        });

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
    File(String),
    Pipe(Command),
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
    pub to: OutputStream,
}

impl Redirect {
    pub fn new_pipe(command: Command) -> Redirect {
        Self {
            from: OutputStream::Stdout,
            redirect_type: RedirectType::Overwrite,
            to: OutputStream::Pipe(command),
        }
    }

    pub fn open_output(&self) -> io::Result<fs::File> {
        let filename = match &self.to {
            OutputStream::File(filename) => filename,
            output => unimplemented!("open output for {:?}", output),
        };

        Ok(match self.redirect_type {
            RedirectType::Overwrite => fs::File::create(filename)?,
            RedirectType::Append => fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(filename)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case(r#"hello    world"#, Command::new(vec!["hello", "world"], vec![]))]
    #[case(r#"'hello    world'"#, Command::new(vec!["hello    world"], vec![]))]
    #[case(r#"'hello''world'"#, Command::new(vec!["helloworld"], vec![]))]
    #[case(r#"hello''world"#, Command::new(vec!["helloworld"], vec![]))]
    #[case(r#""hello    world""#, Command::new(vec!["hello    world"], vec![]))]
    #[case(r#""hello""world""#, Command::new(vec!["helloworld"], vec![]))]
    #[case(r#""hello" "world""#, Command::new(vec!["hello", "world"], vec![]))]
    #[case(r#""shell's test""#, Command::new(vec!["shell's test"], vec![]))]
    #[case(r#"echo three\ \ \ spaces"#, Command::new(vec!["echo", "three   spaces"], vec![]))]
    #[case(r#"echo before\  after"#, Command::new(vec!["echo", "before ", "after"], vec![]))]
    #[case(r#"echo test\nexample"#, Command::new(vec!["echo", "testnexample"], vec![]))]
    #[case(r#"echo hello\\world"#, Command::new(vec!["echo", r#"hello\world"#], vec![]))]
    #[case(r#"echo \'hello\'"#, Command::new(vec!["echo", "'hello'"], vec![]))]
    #[case(r#"echo 'shell\\\nscript'"#, Command::new(vec!["echo", r#"shell\\\nscript"#], vec![]))]
    #[case(r#"echo 'example\"test'"#, Command::new(vec!["echo", r#"example\"test"#], vec![]))]
    #[case(r#"echo 'world\"testhello\"shell'"#, Command::new(vec!["echo", r#"world\"testhello\"shell"#], vec![]))]
    #[case(r#"echo "hello'test'\\'script""#, Command::new(vec!["echo", r#"hello'test'\'script"#], vec![]))]
    #[case(r#"cat "/tmp/fox/\"f 32\"""#, Command::new(vec!["cat", r#"/tmp/fox/"f 32""#], vec![]))]
    #[case(r#"cat "/tmp/fox/\"f\\87\"""#, Command::new(vec!["cat", r#"/tmp/fox/"f\87""#], vec![]))]
    #[case(r#"cat "/tmp/fox/f17""#, Command::new(vec!["cat", "/tmp/fox/f17"], vec![]))]
    #[case(r#"'my program' argument1"#, Command::new(vec!["my program", "argument1"], vec![]))]
    #[case(r#""exe with spaces" file.txt"#, Command::new(vec!["exe with spaces", "file.txt"], vec![]))]
    #[case(r#"'exe with "quotes"' file"#, Command::new(vec![r#"exe with "quotes""#, "file"], vec![]))]
    #[case(r#""exe with 'single quotes'" file"#, Command::new(vec!["exe with 'single quotes'", "file"], vec![]))]
    #[case(r#"'exe with \n newline' arg"#, Command::new(vec![r#"exe with \n newline"#, "arg"], vec![]))]
    #[case("echo hello > output.txt", Command::new(vec!["echo", "hello"], vec![Redirect{
        from: OutputStream::default(),
        redirect_type: RedirectType::Overwrite,
        to: OutputStream::File(String::from("output.txt")),
    }]))]
    #[case("echo hello 1> file\\ txt", Command::new(vec!["echo", "hello"], vec![Redirect{
        from: OutputStream::Stdout,
        redirect_type: RedirectType::Overwrite,
        to: OutputStream::File(String::from("file txt")),
    }]))]
    #[case("echo hello 1>fi''le.txt", Command::new(vec!["echo", "hello"], vec![Redirect{
        from: OutputStream::Stdout,
        redirect_type: RedirectType::Overwrite,
        to: OutputStream::File(String::from("file.txt")),
    }]))]
    #[case("echo 'Hello Alice' 1>> file", Command::new(vec!["echo", "Hello Alice"], vec![Redirect{
        from: OutputStream::Stdout,
        redirect_type: RedirectType::Append,
        to: OutputStream::File(String::from("file")),
    }]))]
    #[case("cat /tmp/foo/file | wc", Command::new(vec!["cat", "/tmp/foo/file"], vec![
        Redirect::new_pipe(Command::new(vec!["wc"], vec![]))
    ]))]
    #[case("cat /tmp/foo/file | head -n 3 | wc", Command::new(vec!["cat", "/tmp/foo/file"], vec![
        Redirect::new_pipe(Command::new(
            vec!["head", "-n", "3"],
            vec![
                Redirect::new_pipe(Command::new(vec!["wc"], vec![]))
            ],
        ))
    ]))]
    fn parser_test(#[case] input: &str, #[case] expected: Command) {
        let mut parser = Parser::new(String::from(input));
        let command = parser.parse();
        assert_eq!(command, expected);
    }
}
