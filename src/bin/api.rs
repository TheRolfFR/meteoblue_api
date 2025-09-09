use std::fs::File;
use rouille::{self, Request, Response, router};
use tempfile::Builder;
use tokio::runtime::Runtime;

use meteoblue_api::forecast::get_forecast_from_url;
use meteoblue_api::screenshot::{ScreenshotParams, capture_screenshot};

fn extract_url(request: &Request) -> Option<String> {
    let raw_query = request.raw_query_string();
    raw_query.split("url=").nth(1).map(|str| str.to_owned())
}

fn route_request(request: &Request) -> Response {
    
    router!(request,
        (GET) ["/ping"] => {
            rouille::Response::text("pong")
        },
        (GET) ["/forecast"] => {
            let url = if let Some(url) = extract_url(request) {
                url
            } else {
                return rouille::Response::empty_400()
            };
            let response = get_forecast_from_url(&url);
            rouille::Response::json(&response)
        },
        (GET) ["/graph"] => {
            let url = if let Some(url) = extract_url(request) {
                url
            } else {
                return rouille::Response::empty_400()
            };
            let darkmode: bool = request.get_param("darkmode").is_some();
            let transparent: bool = request.get_param("transparent").is_some();

            let res_tempfile_path_string = Builder::new()
                .prefix("graph").suffix(".png").tempfile()
                .map(|file| file.path().to_string_lossy().into_owned())
                .map_err(|_| rouille::Response::empty_400().with_status_code(500));
            
            let tempfile_path_string = match res_tempfile_path_string {
                Ok(p) => p,
                Err(e) => {
                    return e;
                } 
            };

            let screenshot_params = ScreenshotParams {
                url,
                darkmode,
                transparent,
                headless: true,
                output: tempfile_path_string.clone(),
            };

            // Process with proper resource cleanup
            let rt = Runtime::new().unwrap();
            let result = rt.block_on(async {
                capture_screenshot(screenshot_params).await
            });

            let response = result.and_then(|_| {
                let file = File::open(&tempfile_path_string)?;
                Ok(rouille::Response::from_file("image/png", file))
            })
            .unwrap_or_else(|err| rouille::Response::text(err.to_string()).with_status_code(500));

            // delete file anyway
            std::fs::remove_file(&tempfile_path_string).ok();  // try

            response
        },
        _ => rouille::Response::empty_404()
    )
}

fn main() {
    let addr = format!("localhost:{}", std::env::var("PORT").unwrap_or("4242".to_string()));
    println!("Starting server on address {addr}");
    rouille::start_server(addr, |request| route_request(request))
}
