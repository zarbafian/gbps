use slog::{o, Drain, Logger};

pub fn terminal_logger() -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    Logger::root(drain, o!())
}

#[cfg(test)]
mod tests {
    use slog::info;

    use super::*;

    #[test]
    fn test_configure_logger() {

        let logger = terminal_logger();

        info!(logger, "Testing configure_logger()...");
    }
}