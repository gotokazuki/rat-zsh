use indicatif::ProgressStyle;

pub fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("\x1b[33m{spinner}\x1b[0m {wide_msg}")
        .unwrap()
        .tick_strings(&["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"])
}

pub fn ok_style() -> ProgressStyle {
    ProgressStyle::with_template("\x1b[32m✔\x1b[0m {wide_msg}").unwrap()
}

pub fn err_style() -> ProgressStyle {
    ProgressStyle::with_template("\x1b[31m✘\x1b[0m {wide_msg}").unwrap()
}
