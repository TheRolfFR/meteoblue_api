use std::fs::File;
use rouille::{self, Request, Response, input, router};
use tempfile::Builder;
use tokio::runtime::Runtime;

use meteoblue_api::log;
use meteoblue_api::forecast::get_forecast_from_url;
use meteoblue_api::screenshot::{ScreenshotParams, full_screenshot_process};
use meteoblue_api::screenshot_34::ThirtyFourEngine;

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
            log::debug!("New graph request: {:?}", &request);
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
                headless: std::env::var("HEADLESS").is_ok_and(|v| v.to_lowercase() == "false"),
                output: tempfile_path_string.clone(),
            };

            // Process with proper resource cleanup
            let rt = Runtime::new().unwrap();
            let opt_timeout = std::env::var("TIMEOUT").ok().and_then(|s| s.parse().ok());
            let result = rt.block_on(async {
                ThirtyFourEngine::full_screenshot_process(screenshot_params, opt_timeout).await
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

struct SingleAuthHandler {
    login: String,
    password: String
}

impl SingleAuthHandler {
    pub fn new(login: String, password: String) -> Self {
        Self { login, password }
    }
    pub fn auth_and_handle(&self, request: &Request) -> Response {
        let auth =  match input::basic_http_auth(request) {
            Some(a) => a,
            None => return Response::basic_http_auth_login_required("realm")
        };

        if auth.login != self.login || auth.password != self.password {
            return Response::empty_400().with_status_code(403);
        }

        route_request(request)
    }
}

fn main() {
    log::init();

    let port =  std::env::var("PORT").unwrap_or("4242".to_string());
    let iface = std::env::var("IFACE").unwrap_or("localhost".to_string());

    let login_expected = std::env::var("LOGIN").expect("LOGIN required");
    let password_expected: String = std::env::var("PASSWORD").expect("PASSWORD required");
    let handler = SingleAuthHandler::new(login_expected, password_expected);

    let addr = format!("{iface}:{port}");
    println!("Starting server on address {addr}");
    rouille::start_server(addr, move |request| handler.auth_and_handle(request))
}
