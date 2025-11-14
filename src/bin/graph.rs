use clap::{Parser};
use meteoblue_api::screenshot::{ScreenshotParams, full_screenshot_process};
use meteoblue_api::log;
use tokio::runtime::Runtime;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: ScreenshotParams = ScreenshotParams::parse();

    log::init();

    let rt = Runtime::new().unwrap();
    rt.block_on(full_screenshot_process(opts))
}
