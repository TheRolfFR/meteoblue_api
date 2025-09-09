use clap::{Parser};
use meteoblue_api::screenshot::{ScreenshotParams, capture_screenshot};
use tokio::runtime::Runtime;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: ScreenshotParams = ScreenshotParams::parse();

    let rt = Runtime::new().unwrap();
    rt.block_on(capture_screenshot(opts))
}
