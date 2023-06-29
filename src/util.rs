#[macro_export]
macro_rules! failf {
    ($format_string:expr) => {{
        eprintln!($format_string);
        std::process::exit(1)
    }};
    ($format_string:expr, $($arg:expr),* $(,)?) => {{
        eprintln!($format_string, $($arg),*);
        std::process::exit(1)
    }};
}
