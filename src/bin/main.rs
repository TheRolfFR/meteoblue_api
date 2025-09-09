use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use meteoblue_api::forecast::{load_from_url, get_forecast_from_html};

fn load_from_file<T: AsRef<Path>>(file_path: T) -> std::io::Result<String> {
    let mut file = File::open(file_path)?;
    let mut html_content = String::new();
    file.read_to_string(&mut html_content)?;

    Ok(html_content)
}


fn main() -> std::io::Result<()>  {
    let args = std::env::args();
    let url_opt = args.skip(1).next();
    let html_content = if let Some(url) = url_opt {
        load_from_url(&url).unwrap()
    } else {
        load_from_file("content.html").unwrap()
    };

    let hourly_forecast = get_forecast_from_html(&html_content);
    println!("{}", serde_json::to_string_pretty(&hourly_forecast).unwrap());

    Ok(())
}
