pub use log::*;
use env_logger::{self, Env};
use std::io::Write;

pub fn init() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn"))
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
        // .filter(Some("playwright"), LevelFilter::Warn)
        .init();
}
