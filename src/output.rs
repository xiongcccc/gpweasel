use std::{
    io::{self, IsTerminal, Write},
    sync::atomic::{AtomicUsize, Ordering},
};

static PAGE_SIZE: AtomicUsize = AtomicUsize::new(0);
static LINES_WRITTEN: AtomicUsize = AtomicUsize::new(0);

pub fn init(page_size: Option<usize>) {
    PAGE_SIZE.store(page_size.unwrap_or(0), Ordering::Relaxed);
    LINES_WRITTEN.store(0, Ordering::Relaxed);
}

pub fn line(args: std::fmt::Arguments<'_>) {
    let result = {
        let mut stdout = io::stdout().lock();
        stdout.write_fmt(args).and_then(|_| stdout.write_all(b"\n"))
    };

    if let Err(err) = result {
        if err.kind() == io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
        panic!("failed printing to stdout: {err}");
    }

    maybe_pause();
}

fn maybe_pause() {
    let page_size = PAGE_SIZE.load(Ordering::Relaxed);
    if page_size == 0 {
        return;
    }

    let line_count = LINES_WRITTEN.fetch_add(1, Ordering::Relaxed) + 1;
    if line_count % page_size != 0 || !io::stdout().is_terminal() || !io::stdin().is_terminal() {
        return;
    }

    let mut stderr = io::stderr().lock();
    if write!(stderr, "--More--").and_then(|_| stderr.flush()).is_err() {
        return;
    }

    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    let _ = write!(stderr, "\r        \r").and_then(|_| stderr.flush());

    if input.trim().eq_ignore_ascii_case("q") {
        std::process::exit(0);
    }
}

#[macro_export]
macro_rules! outln {
    ($($arg:tt)*) => {{
        $crate::output::line(format_args!($($arg)*));
    }};
}
