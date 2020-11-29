use std::collections::HashMap;
use std::error::Error;

pub fn configure_logging(filename: String, level: String) -> Result<(), Box<dyn Error>>{

    use log::{LevelFilter};
    use log4rs::append::file::FileAppender;
    use log4rs::encode::pattern::PatternEncoder;
    use log4rs::config::{Appender, Config, Root};
    use log4rs::append::console::ConsoleAppender;

    let mut log_levels: HashMap<String, LevelFilter> = HashMap::new();
    log_levels.insert(String::from("OFF"), LevelFilter::Off);
    log_levels.insert(String::from("ERROR"), LevelFilter::Error);
    log_levels.insert(String::from("WARN"), LevelFilter::Warn);
    log_levels.insert(String::from("INFO"), LevelFilter::Info);
    log_levels.insert(String::from("DEBUG"), LevelFilter::Debug);
    log_levels.insert(String::from("TRACE"), LevelFilter::Trace);

    let mut level_filter = &LevelFilter::Info;

    if let Some(filter) = log_levels.get(&level.to_uppercase()) {
        level_filter = filter;
    }
    else {
        Err(format!("Invalid logging level: {}", level))?
    }

    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} [{l}] {T} - {m}{n}")))
        .build(filename)?;

    let console = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("[{l}] {T} - {m}{n}")))
        .build();

    let config = Config::builder()
        .appender(Appender::builder().build("console", Box::new(console)))
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder()
            .appender("console")
            .appender("logfile")
            .build(*level_filter))?;

    log4rs::init_config(config)?;

    Ok(())
}