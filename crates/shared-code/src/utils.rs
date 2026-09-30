pub fn print_error(error: &dyn core::error::Error) {
    eprintln!("{error}");

    let mut maybe_source = error.source();
    while let Some(source) = maybe_source.take() {
        let error_msg = source.to_string().replace("\n", "\n\t\t");
        eprintln!("\toccured because of: {error_msg}");
        maybe_source = source.source();
    }
}
