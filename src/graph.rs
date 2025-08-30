use std::io;
use std::io::prelude::*;

use clap::{Parser};
use playwright::{Playwright, api::{Cookie, ScreenshotType, Viewport}};
use tokio::runtime::Runtime;

/// CLI tool to take screenshots of current day forecast with headless browser
#[derive(Parser, Debug)]
struct Opts {
    /// URL of the page to capture screenshot from.
    #[clap()]
    url: String,

    /// Run browser in headless mode (default: true).
    #[clap(long, action = clap::ArgAction::Set, default_value_t = true)]
    headless: bool,

    /// Get in darkmode (default: false).
    #[clap(long, action = clap::ArgAction::Set, default_value_t = false)]
    darkmode: bool,

    /// Get screenshot transparent (default: false).
    #[clap(long, action = clap::ArgAction::Set, default_value_t = false)]
    transparent: bool,

    /// Output path for the screenshot (optional).
    #[clap(short, long, default_value = "screenshot.png")]
    output: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    let rt = Runtime::new().unwrap();
    rt.block_on(async_main(opts))
}

async fn async_main(opts: Opts) -> Result<(), Box<dyn std::error::Error>> {
    println!("{:?}", opts);
    let playwright = Playwright::initialize().await?;
    playwright.prepare()?;

    let chromium = playwright.chromium();
    let browser = chromium.launcher().headless(opts.headless).launch().await?;
    let context = browser.context_builder().viewport(Some(Viewport {
        width: 1280,
        height: 720
    })).build().await?;

    let cookies = [
        Cookie {
            name: "extendview".into(),
            value: "true".into(),
            url: Some(opts.url.clone().into()),
            domain: None,
            path: None,
            expires: None,
            http_only: None,
            secure: None,
            same_site: None
        },
        Cookie {
            name: "darkmode".into(),
            value: opts.darkmode.to_string(),
            url: Some(opts.url.clone().into()),
            domain: None,
            path: None,
            expires: None,
            http_only: None,
            secure: None,
            same_site: None
        },
    ];
    context.add_cookies(&cookies).await?;

    let page = context.new_page().await?;
    page.goto_builder(&opts.url).goto().await?;

    println!("Looking for graph at #hourly_forecast...");
    page.wait_for_selector_builder("#hourly_forecast")
        .wait_for_selector().await?;

    let hourly_forecast = page.query_selector("#hourly_forecast").await?.unwrap();

    // remove cookie popup
    println!("Looking for cookie consent popup...");
    if let Ok(Some(_)) = page.wait_for_selector_builder(".fc-consent-root")
        .wait_for_selector().await {
            page.evaluate::<_, ()>("
                Element.prototype.remove = function() {
                    this.parentElement.removeChild(this)
                }
                document.querySelector('.fc-consent-root').remove()", 
                serde_json::json!({})).await?;
            println!("Removed concent popup");
        }
    else {
        println!("No cookie consent popup found");
    }

    // for full screenshot scroll to element
    hourly_forecast.scroll_into_view_if_needed(None).await?;
    // Scroll down by 10px
    page.evaluate::<_, ()>("window.scrollBy({ top: 10, left: 0, behavior: \"instant\" });", serde_json::json!({})).await?;

    let mut graph_rect = hourly_forecast.bounding_box().await?.unwrap();
    graph_rect.width += 5.; // add some margin right
    graph_rect.height += 5.; // add some margin below

    if opts.transparent {
        // Inject CSS to remove background from the entire page or specific elements
        let css = "
            body {
                background: none !important;
            }
            /* Add more styles as needed */
        ";

        page.add_style_tag(css, None).await?;

        println!("CSS injected successfully!");
    }
    

    if !opts.headless {
        pause();
    }

    println!("Building screenshot at {}...", &opts.output);
    page.screenshot_builder()
        .r#type(ScreenshotType::Png)
        .clear_type()
        .omit_background(opts.transparent)
        .path((&opts.output.clone()).into())
        .clip(graph_rect)
        .screenshot()
        .await?;

    println!("Exiting...");
    Ok(())
}

fn pause() {
    // If not running in headless mode, pause execution until browser is closed manually
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    // We want the cursor to stay at the end of the line, so we print without a newline and flush manually.
    write!(stdout, "Press any key to continue...").unwrap();
    stdout.flush().unwrap();

    // Read a single byte and discard
    let _ = stdin.read(&mut [0u8]).unwrap();
}
