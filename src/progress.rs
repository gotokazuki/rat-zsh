use indicatif::ProgressStyle;

/// Spinner style used during ongoing operations.
/// - Yellow spinner with animated braille-style frames.
/// - Displays the current message (`{wide_msg}`) next to the spinner.
pub fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("\x1b[33m{spinner}\x1b[0m {wide_msg}")
        .unwrap()
        .tick_strings(&["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"])
}

/// Style used when an operation finishes successfully.
/// - Green check mark followed by the final message.
pub fn ok_style() -> ProgressStyle {
    ProgressStyle::with_template("\x1b[32m✔\x1b[0m {wide_msg}").unwrap()
}

/// Style used when an operation fails with an error.
/// - Red cross followed by the error message.
pub fn err_style() -> ProgressStyle {
    ProgressStyle::with_template("\x1b[31m✘\x1b[0m {wide_msg}").unwrap()
}
