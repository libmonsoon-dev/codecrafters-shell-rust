pub struct Lexer {
    input: Vec<char>,
    position: usize,
}

impl Lexer {
    pub fn new(input: String) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
        }
    }

    pub fn lex(&mut self) -> Vec<Token> {
        let mut tokens: Vec<Token> = Vec::new();
        while !self.is_eof() {
            tokens.push(self.next_token());
        }
        tokens.push(Token::eof(self.position));
        tokens
    }

    fn next_token(&mut self) -> Token {
        let token = match self.input[self.position] {
            '\'' => self.handle_single_quote(),
            char if is_string_char(char) => self.handle_string(),
            char if char::is_whitespace(char) => self.handle_whitespace(),
            char @ _ => unimplemented!("handling of {:?}", char),
        };

        token
    }

    fn is_eof(&self) -> bool {
        self.position >= self.input.len()
    }

    fn handle_single_quote(&mut self) -> Token {
        let lexeme = String::from(self.input[self.position]);
        self.position += 1;

        Token {
            kind: TokenKind::SingleQuote,
            lexeme,
        }
    }

    fn handle_string(&mut self) -> Token {
        let mut end_position = self.position;
        while end_position < self.input.len() && is_string_char(self.input[end_position]) {
            end_position += 1;
        }
        let lexeme: String = self.input[self.position..end_position].iter().collect();
        self.position = end_position;

        Token {
            kind: TokenKind::String,
            lexeme,
        }
    }

    fn handle_whitespace(&mut self) -> Token {
        let mut end_position = self.position;
        while end_position < self.input.len() && char::is_whitespace(self.input[end_position]) {
            end_position += 1;
        }
        let lexeme: String = self.input[self.position..end_position].iter().collect();
        self.position = end_position;

        Token {
            kind: TokenKind::Whitespace,
            lexeme,
        }
    }
}

fn is_string_char(char: char) -> bool {
    char == '/' || char::is_alphanumeric(char)
}

#[derive(PartialEq, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    // pub span: Span,
}

impl Token {
    fn eof(_position: usize) -> Self {
        Self {
            kind: TokenKind::EOF,
            lexeme: String::new(),
            // span: Span {
            //     start: position,
            //     end: position,
            // },
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum TokenKind {
    SingleQuote,
    String,
    Whitespace,
    EOF,
}

// #[derive(PartialEq, Debug)]
// pub struct Span {
//     pub start: usize,
//     pub end: usize,
// }

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn simple() {
        let mut lexer = Lexer::new(String::from(r#"hello    world"#));
        let tokens = lexer.lex();
        assert_eq!(
            tokens,
            vec![
                Token {
                    kind: TokenKind::String,
                    lexeme: String::from("hello"),
                    // span: Span { start: 0, end: 4 },
                },
                Token {
                    kind: TokenKind::Whitespace,
                    lexeme: String::from("    "),
                    // span: Span { start: 0, end: 4 }, //TODO
                },
                Token {
                    kind: TokenKind::String,
                    lexeme: String::from("world")
                },
                Token {
                    kind: TokenKind::EOF,
                    lexeme: String::new(),
                }
            ]
        );
    }

    #[test]
    fn spaces_within_quotes() {
        let mut lexer = Lexer::new(String::from(r#"'hello    world'"#));
        let tokens = lexer.lex();
        assert_eq!(
            tokens,
            vec![
                Token {
                    kind: TokenKind::SingleQuote,
                    lexeme: String::from("'")
                },
                Token {
                    kind: TokenKind::String,
                    lexeme: String::from("hello")
                },
                Token {
                    kind: TokenKind::Whitespace,
                    lexeme: String::from("    ")
                },
                Token {
                    kind: TokenKind::String,
                    lexeme: String::from("world")
                },
                Token {
                    kind: TokenKind::SingleQuote,
                    lexeme: String::from("'")
                },
                Token {
                    kind: TokenKind::EOF,
                    lexeme: String::new(),
                }
            ]
        );
    }

    #[test]
    fn empty_input() {
        let mut lexer = Lexer::new(String::from(""));
        let tokens = lexer.lex();
        assert_eq!(
            tokens,
            vec![Token {
                kind: TokenKind::EOF,
                lexeme: String::from("")
            },]
        );
    }
}
