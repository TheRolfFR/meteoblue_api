use std::io;
use std::io::prelude::*;

use clap::{Parser};
use playwright::{Playwright, api::{Browser, BrowserContext, BrowserType, Cookie, Page, ScreenshotType, Viewport}};
use crate::log;

/// CLI tool to take screenshots of current day forecast with headless browser
#[derive(Parser, Debug)]
pub struct ScreenshotParams {
    /// URL of the page to capture screenshot from.
    #[clap()]
    pub url: String,

    /// Run browser in headless mode (default: true).
    #[clap(long, action = clap::ArgAction::Set, default_value_t = true)]
    pub headless: bool,

    /// Get in darkmode (default: false).
    #[clap(long, action = clap::ArgAction::Set, default_value_t = false)]
    pub darkmode: bool,

    /// Get screenshot transparent (default: false).
    #[clap(long, action = clap::ArgAction::Set, default_value_t = false)]
    pub transparent: bool,

    /// Output path for the screenshot (optional).
    #[clap(short, long, default_value = "screenshot.png")]
    pub output: String,
}

/// Very important to keep all together not to have ObjectNotFound error
pub struct PlayWrightInstance {
    /// The Playwright instance used to control the browser
    _playwright: Playwright,
    /// The browser type (e.g., chromium, firefox, webkit)
    _browser_type: BrowserType,
    /// The actual browser instance
    _browser: Browser,
    /// The browser context for managing sessions and cookies
    pub context: BrowserContext,
}

#[inline(always)]
pub async fn prepare_browser(headless: bool) -> Result<PlayWrightInstance, &'static str> {
    log::debug!("Opening browser...");

    let playwright = Playwright::initialize().await.map_err(|_| "Failed to initialize")?;
    playwright.prepare().map_err(|_| "Faield to prepare browser")?;

    let chromium = playwright.chromium();
    let browser = chromium.launcher().headless(headless).launch().await.map_err(|_| "Failed to launch browser")?;
    let context = browser.context_builder().viewport(Some(Viewport {
        width: 1280,
        height: 720
    })).build().await.map_err(|_| "Failed to build viewport")?;

    Ok(PlayWrightInstance { _playwright: playwright, _browser_type: chromium, _browser: browser, context })
}

#[inline(always)]
pub async fn store_cookie(context: &BrowserContext, opts: &ScreenshotParams) -> Result<(), &'static str> {
    log::debug!("Storing cookies...");
    if let Err(e) = context.clear_cookies().await {
        let err_msg = "Failed to clear cookies";
        log::error!("{}: {}", err_msg, e);
        return Err(err_msg);
    }

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

    if let Err(e) = context.add_cookies(&cookies).await {
        let err_msg = "Failed add cookies";
        log::error!("{}: {}", err_msg, e);
        return Err(err_msg);
    }

    Ok(())
}

pub async fn navigate_and_capture_screenshot(page: &Page, opts: &ScreenshotParams) -> Result<(), &'static str> {
    page.goto_builder(&opts.url).goto().await.map_err(|_| "Failed to go to url")?;

    log::debug!("Looking for graph at #hourly_forecast...");
    page.wait_for_selector_builder("#hourly_forecast")
        .wait_for_selector().await.map_err(|_| "Failed to find hourly forecast element")?;

    let opt_hourly_forecast = page.query_selector("#hourly_forecast").await.map_err(|_| "Failed to fetch handle")?;
    let hourly_forecast = opt_hourly_forecast.ok_or("Failed to find hourly forecast")?;

    // remove cookie popup
    log::debug!("Looking for cookie consent popup...");
    if let Ok(Some(_)) = page.wait_for_selector_builder(".fc-consent-root")
        .timeout(2.0)
        .wait_for_selector().await {
            page.evaluate::<_, ()>("
                Element.prototype.remove = function() {
                    this.parentElement.removeChild(this)
                }
                document.querySelector('.fc-consent-root').remove()", 
                serde_json::json!({})).await.map_err(|_| "Failed to remove consent popup")?;
            log::debug!("Removed concent popup");
        }
    else {
        log::debug!("No cookie consent popup found");
    }

    // for full screenshot scroll to element
    hourly_forecast.scroll_into_view_if_needed(None).await.map_err(|_| "Failed to scroll into graph")?;
    // Scroll down by 10px
    page.evaluate::<_, ()>("window.scrollBy({ top: 10, left: 0, behavior: \"instant\" });", serde_json::json!({}))
        .await.map_err(|_| "Failed to scroll more into graph")?;

    let opt_graph_rect = hourly_forecast.bounding_box().await.map_err(|_| "Failed to fetch forecast bounding box")?;
    let mut graph_rect = opt_graph_rect.ok_or("Bounding box found None")?;
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

        page.add_style_tag(css, None).await.map_err(|_| "Failed to inject styles")?;

        log::debug!("CSS injected successfully!");
    }
    

    if !opts.headless {
        pause();
    }

    log::debug!("Building screenshot at {}...", &opts.output);
    page.screenshot_builder()
        .r#type(ScreenshotType::Png)
        .clear_type()
        .omit_background(opts.transparent)
        .path((&opts.output.clone()).into())
        .clip(graph_rect)
        .screenshot()
        .await.map_err(|_| "Failed to make screenshot")?;

    Ok(())
}

pub async fn full_screenshot_process(opts: ScreenshotParams) -> Result<(), Box<dyn std::error::Error>> {
    log::debug!("{:?}", opts);

    let instance = prepare_browser(opts.headless).await?;
    store_cookie(&instance.context, &opts).await?;

    let page = instance.context.new_page().await?;
    navigate_and_capture_screenshot(&page, &opts).await?;

    log::debug!("Exiting...");
    Ok(())
}


fn pause() {
    // If not running in headless mode, pause execution until browser is closed manually
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    // We want the cursor to stay at the end of the line, so we print without a newline and flush manually.
    write!(stdout, "Press any key to continue...").expect("Shall press key");
    stdout.flush().expect("Failed to flush");

    // Read a single byte and discard
    let _ = stdin.read(&mut [0u8]).expect("Failed to read byte for pause");
}
