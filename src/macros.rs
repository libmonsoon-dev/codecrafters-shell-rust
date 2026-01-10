#[macro_export]
macro_rules! print_to {
    ($out:expr, $fmt:expr) => {{
        $out.write_fmt(format_args!($fmt)).unwrap();
    }};
    ($out:expr, $fmt:expr, $($args:tt)*) => {{
        $out.write_fmt(format_args!($fmt, $($args)*)).unwrap();
    }};
}

#[macro_export]
macro_rules! print {
    ($fmt:expr) => {{
        use std::io::Write;

        crate::print_to!(std::io::stdout(), $fmt);
    }};
    ($fmt:expr, $($args:tt)*) => {{
        use std::io::Write;

        crate::print_to!(std::io::stdout(), $fmt, $($args)*);
    }};
}
