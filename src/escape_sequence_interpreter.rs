pub struct Interpreter<'a> {
    input: &'a String,
}

impl<'a> Interpreter<'a> {
    pub fn new(input: &'a String) -> Self {
        Self { input }
    }

    pub fn interpret(&mut self) -> String {
        if !self.input.contains('\\') {
            return self.input.clone();
        }

        let mut output = String::with_capacity(self.input.len());
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
        let input = String::from("hello");
        let mut interpreter = Interpreter::new(&input);
        assert_eq!(interpreter.interpret(), String::from("hello"));
    }

    #[test]
    fn each_backslash_creates_a_literal_space_as_part_of_one_argument() {
        let input = String::from(r#"three\ \ \ spaces"#);
        let mut interpreter = Interpreter::new(&input);
        assert_eq!(interpreter.interpret(), String::from("three   spaces"));
    }

    #[test]
    fn the_backslash_preserves_the_first_space_literally_but_the_shell_collapses_the_subsequent_unescaped_spaces()
     {
        let input = String::from(r#"before\ "#);
        let mut interpreter = Interpreter::new(&input);
        assert_eq!(interpreter.interpret(), String::from("before "));
    }

    #[test]
    fn backslash_n_becomes_just_n() {
        let input = String::from(r#"test\nexample"#);
        let mut interpreter = Interpreter::new(&input);
        assert_eq!(interpreter.interpret(), String::from("testnexample"));
    }

    #[test]
    fn the_first_backslash_escapes_the_second() {
        let input = String::from(r#"hello\\world"#);
        let mut interpreter = Interpreter::new(&input);
        assert_eq!(interpreter.interpret(), String::from(r#"hello\world"#));
    }

    #[test]
    fn backslash_quote_makes_the_quote_literal_character() {
        let input = String::from(r#"\'hello\'"#);
        let mut interpreter = Interpreter::new(&input);
        assert_eq!(interpreter.interpret(), String::from("'hello'"));
    }
}
