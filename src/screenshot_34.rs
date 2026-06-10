use std::time::Duration;
use std::path::Path;

use thirtyfour::prelude::*;

use crate::pause::pause;
use crate::log;
use crate::screenshot::ScreenshotParams;

pub struct ThirtyFourEngine {
    driver: WebDriver,
}

impl ThirtyFourEngine {
    async fn cropped_screenshot<P>(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        path: impl AsRef<Path>,
    ) -> Result<(), &'static str> {
        log::info!("Screenshotting webdriver");
        self.driver.screenshot(path.as_ref()).await.map_err(|_| "Failed to save screenshot")?;

        let truepath = path.as_ref().to_path_buf();
        let copy_path = truepath.with_file_name("screenshot_full.png");

        std::fs::copy(&truepath, &copy_path).ok();
        println!("{:?}", copy_path);

        tokio::task::spawn_blocking(move || {
            // open up the image file
            log::info!("Reading image from file");
            let mut img = image::open(&truepath).map_err(|_| "Cannot open original screenshot")?;

            // crop the image
            log::info!("Cropping image");
            let cropped_img = image::imageops::crop(&mut img, x, y, width, height).to_image();

            // save the image
            log::info!("Writing image to file");
            cropped_img.save(&truepath).map_err(|e| {
                let err_msg = "Cannot save cropped screenshot";
                log::error!("{}: {}", err_msg, e);
                err_msg
            })?;

            Result::<(), &'static str>::Ok(())
        })
        .await.map_err(|_| "Failed to join thread")??;
        Ok(())
    }

    async fn prepare_browser(headless: bool) -> Result<Self, &'static str>
    {
        let mut caps = DesiredCapabilities::chrome();

        if headless {
            caps.set_headless().map_err(|_| "Failed to change headless")?;
        } else {
            caps.unset_headless().map_err(|_| "Failed to change headless")?;
        }

        let driver = WebDriver::managed(caps).await.map_err(|e| {
            let err_msg = "Failed to initialize";
            log::error!("{}: {}", err_msg, e);
            return err_msg;
        })?;

        driver.set_window_rect(0, 0, 1280, 1600).await.map_err(|_| "Failed to build viewport")?;

        Ok(ThirtyFourEngine {
            driver
        })
    }

    async fn store_cookie(&mut self, opts: &ScreenshotParams) -> Result<(), &'static str>
    {
        self.driver.delete_all_cookies().await.map_err(|e| {
            let err_msg = "Failed to clear cookies";
            log::error!("{}: {}", err_msg, e);
            return err_msg;
        })?;

        let cookies = [
            Cookie {
                name: "extendview".into(),
                value: "true".into(),
                domain: None,
                path: None,
                expiry: None,
                http_only: None,
                secure: None,
                same_site: None
            },
            Cookie {
                name: "darkmode".into(),
                value: opts.darkmode.to_string(),
                domain: None,
                path: None,
                expiry: None,
                http_only: None,
                secure: None,
                same_site: None
            }
        ];

        for cookie in cookies {
            self.driver.add_cookie(cookie).await.map_err(|e| {
                let err_msg = "Failed add cookies";
                log::error!("{}: {}", err_msg, e);
                return err_msg;
            })?;
        }

        Ok(())
    }

    async fn navigate(&mut self, opts: &ScreenshotParams) -> Result<(), &'static str>
    {
        self.driver.goto(&opts.url).await.map_err(|_| "Failed to go to url")?;
        Ok(())
    }

    async fn capture_screenshot(&mut self, opts: &ScreenshotParams, opt_consent_timeout: Option<f64>) -> Result<(), &'static str>
    {
        self.driver.refresh().await.map_err(|_| "Failed to refresh page!")?;

        log::debug!("Looking for graph at #hourly_forecast...");
        let hourly_forecast = self.driver.find(By::Id("hourly_forecast")).await.map_err(|_| "Failed to find hourly forecast")?;

        log::debug!("Looking for cookie consent popup...");
        let mut popup_query = self.driver.query(By::ClassName("fc-consent-root"));
        if let Some(timeout) = opt_consent_timeout {
            popup_query = popup_query.wait(Duration::from_secs_f64(timeout), Duration::from_millis(20));
        }
        if popup_query.single().await.ok().is_some() {
            self.driver.execute(r#"
                Element.prototype.remove = function() {
                    this.parentElement.removeChild(this)
                }
                document.querySelector('.fc-consent-root').remove();
            "#, Vec::new()).await.map_err(|_| "Failed to remove consent popup")?;
            log::debug!("Removed consent popup");
        }

        // log::debug!("Scroll into graph");
        // hourly_forecast.scroll_into_view().await.map_err(|e| {
        //     let err_msg = "Failed to scroll into graph;";
        //     log::error!("{}: {}", err_msg, e);
        //     err_msg
        // })?;

        if opts.transparent {
            // Inject CSS to remove background from the entire page or specific elements
            let css = r#"
                let style = document.createElement('style');
                style.textContent = `
                    body {
                        background: none !important;
                    }
                `;
                document.head.appendChild(style);
            "#;

            self.driver.execute(css, vec![]).await.map_err(|_| "Failed to inject styles")?;
            log::debug!("CSS injected successfully!");
        }

        if !opts.headless {
            pause();
        }

        log::debug!("Building screenshot at {}...", &opts.output);

        let mut graph_rect = hourly_forecast.rect().await.map_err(|_| "Failed to fetch forecast bounding box")?;

        log::debug!("{:?}", &graph_rect);

        graph_rect.width += 5.; // add some margin right
        graph_rect.height += 5.; // add some margin below

        self.cropped_screenshot::<&String>(
            graph_rect.x.floor() as u32,
            graph_rect.y.floor() as u32,
            graph_rect.width.ceil() as u32,
            graph_rect.height.ceil() as u32,
            &opts.output
        ).await.map_err(|e| {
            let err_msg = "Failed to make screenshot";
            log::error!("{}: {}", err_msg, e);
            err_msg
        })?;

        Ok(())
    }

    pub async fn close(self) -> Result<(), &'static str>{
        // Always explicitly close the browser.
        self.driver.quit().await.map_err(|_| "Failed to close driver")?;
        Ok(())
    }

    pub async fn full_screenshot_process(opts: ScreenshotParams, opt_consent_timeout: Option<f64>) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("{:?}", opts);

        let mut engine = Self::prepare_browser(opts.headless).await?;
        engine.navigate(&opts).await?;
        engine.store_cookie(&opts).await?;
        engine.capture_screenshot(&opts, opt_consent_timeout).await?;
        engine.close().await?;

        log::debug!("Exiting...");
        Ok(())
    }
}
