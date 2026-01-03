#[macro_export]
macro_rules! print {
    ($fmt:expr) => {{
        io::stdout().write_fmt(format_args!($fmt))?;
    }};
    ($fmt:expr, $($args:tt)*) => {{
        io::stdout().write_fmt(format_args!($fmt, $($args)*))?;
    }};
}
