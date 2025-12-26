pub struct Interpreter {
    input: String,
}

impl Interpreter {
    pub fn new(input: String) -> Self {
        Self { input }
    }

    pub fn interpret(&mut self) -> String {
        if !self.input.contains('\\') {
            return self.input.clone();
        }

        let mut output = String::new();
        let mut chars = self.input.chars();

        while let Some(c) = chars.next() {
            if c == '\\' {
                output.push(chars.next().unwrap());
            } else {
                output.push(c);
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn no_slash() {
        let mut interpreter = Interpreter::new(String::from("hello"));
        assert_eq!(interpreter.interpret(), String::from("hello"));
    }

    #[test]
    fn each_backslash_creates_a_literal_space_as_part_of_one_argument() {
        let mut interpreter = Interpreter::new(String::from(r#"three\ \ \ spaces"#));
        assert_eq!(interpreter.interpret(), String::from("three   spaces"));
    }

    #[test]
    fn the_backslash_preserves_the_first_space_literally_but_the_shell_collapses_the_subsequent_unescaped_spaces()
     {
        let mut interpreter = Interpreter::new(String::from(r#"before\ "#));
        assert_eq!(interpreter.interpret(), String::from("before "));
    }

    #[test]
    fn backslash_n_becomes_just_n() {
        let mut interpreter = Interpreter::new(String::from(r#"test\nexample"#));
        assert_eq!(interpreter.interpret(), String::from("testnexample"));
    }

    #[test]
    fn the_first_backslash_escapes_the_second() {
        let mut interpreter = Interpreter::new(String::from(r#"hello\\world"#));
        assert_eq!(interpreter.interpret(), String::from(r#"hello\world"#));
    }

    #[test]
    fn backslash_quote_makes_the_quote_literal_character() {
        let mut interpreter = Interpreter::new(String::from(r#"\'hello\'"#));
        assert_eq!(interpreter.interpret(), String::from("'hello'"));
    }
}
