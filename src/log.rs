pub use log::*;
use env_logger;
use std::io::Write;

pub fn init() {
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{}:{} [{}] - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.level(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Debug)
        // .filter(Some("playwright"), LevelFilter::Warn)
        .init();
}
