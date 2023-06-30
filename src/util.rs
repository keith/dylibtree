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

#[macro_export]
macro_rules! verbose_log {
    ($verbose:expr, $format_string:expr) => {{
        if $verbose {
            eprintln!("VERBOSE: {}", $format_string);
        }
    }};
    ($verbose:expr, $format_string:expr, $($arg:expr),* $(,)?) => {{
        if $verbose {
            eprintln!("VERBOSE: {}", format!($format_string, $($arg),*));
        }
    }};
}
