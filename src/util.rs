pub fn fail<S: Into<String>>(message: S) -> ! {
    eprintln!("{}", message.into());
    std::process::exit(1)
}
