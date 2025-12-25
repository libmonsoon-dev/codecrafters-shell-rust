#[macro_export]
macro_rules! print {
    ($self:expr, $fmt:expr) => {{
        $self.output.write_fmt(format_args!($fmt))?;
    }};
    ($self:expr, $fmt:expr, $($args:tt)*) => {{
        $self.output.write_fmt(format_args!($fmt, $($args)*))?;
    }};
}
