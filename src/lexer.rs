pub struct Lexer {
    input: Vec<char>,
    position: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
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
        tokens.push(Token::eof());
        tokens
    }

    fn next_token(&mut self) -> Token {
        match self.input[self.position] {
            '\'' => self.handle_single_quote(),
            '"' => self.handle_double_quote(),
            '\\' => self.handle_backslash(),
            char if char::is_whitespace(char) => self.handle_whitespace(),
            char if is_string_char(char) => self.handle_string(),
            char => unimplemented!("handling of {:?}", char),
        }
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

    fn handle_double_quote(&mut self) -> Token {
        let lexeme = String::from(self.input[self.position]);
        self.position += 1;

        Token {
            kind: TokenKind::DoubleQuote,
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

    fn handle_backslash(&mut self) -> Token {
        let lexeme: String = self.input[self.position..self.position + 2]
            .iter()
            .collect();
        self.position += 2;

        Token {
            kind: TokenKind::EscapeSequence,
            lexeme,
        }
    }
}

fn is_string_char(char: char) -> bool {
    !['\'', '"', '$', '\\'].contains(&char) && !char::is_whitespace(char)
}

#[derive(PartialEq, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
}

impl Token {
    fn eof() -> Self {
        Self {
            kind: TokenKind::EOF,
            lexeme: String::new(),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum TokenKind {
    SingleQuote,
    DoubleQuote,
    String,
    EscapeSequence,
    Whitespace,
    EOF,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case(r#"hello    world"#, vec![
        Token {
            kind: TokenKind::String,
            lexeme: String::from("hello"),
        },
        Token {
            kind: TokenKind::Whitespace,
            lexeme: String::from("    "),
        },
        Token {
            kind: TokenKind::String,
            lexeme: String::from("world")
        },
        Token {
            kind: TokenKind::EOF,
            lexeme: String::new(),
        }
    ])]
    #[case(r#"'hello    world'"#, vec![
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
    ])]
    #[case("", vec![
        Token {
            kind: TokenKind::EOF,
            lexeme: String::from("")
        }
    ])]
    fn lexer_test(#[case] input: &str, #[case] expected_tokens: Vec<Token>) {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.lex();
        assert_eq!(tokens, expected_tokens,);
    }
}
