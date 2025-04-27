use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use reqwest::{cookie::Jar, Url};
use serde_derive::Serialize;

#[derive(Debug, Default, Serialize)]
struct HourlyForecast {
    hour: u32, // 0-32h

    icon: (String, String), // (Weather icon URL, title)

    temperature: i8, // +-127° should be enough
    windchill: i8, // perceived temperature

    precip_mm: Option<f32>, // Empty string if no precip or real number
    precip_prob: u8, // 0-100%

    winddir: String, // One to three-letter string NNW SE E
    windspeeds: (u8, u8), // (Wind speed, Top wind speed)

    night: bool
}

fn load_from_file<T: AsRef<Path>>(file_path: T) -> std::io::Result<String> {
    let mut file = File::open(file_path)?;
    let mut html_content = String::new();
    file.read_to_string(&mut html_content)?;

    Ok(html_content)
}

fn load_from_url(url: &str) -> Option<String> {

    let cookie = "extendview=true";
    let url = url.parse::<Url>().unwrap();

    let jar = Jar::default();
    jar.add_cookie_str(cookie, &url);

    let http_client = reqwest::blocking::Client::builder()
        .cookie_store(true)
        .cookie_provider(jar.into())
        .build()
        .unwrap();

    let response = http_client.get(url).send();
    response.unwrap().text().ok()
}

fn main() -> std::io::Result<()>  {
    let args = std::env::args();
    let url_opt = args.skip(1).next();
    let html_content = if let Some(url) = url_opt {
        load_from_url(&url).unwrap()
    } else {
        load_from_file("content.html").unwrap()
    };

    let document = scraper::Html::parse_document(&html_content);
    let mut hourly_forecast: [HourlyForecast; 24] = Default::default();

    // times
    let child_selector_str = "#hourly_forecast .times td span";
    let child_selector = scraper::Selector::parse(child_selector_str).unwrap();
    let children = document.select(&child_selector);
    hourly_forecast[0].hour = 0;
    for (i, child) in children.enumerate() {
        let hour_text = child.text().collect::<String>();
        hourly_forecast[i+1].hour = hour_text.trim().parse().unwrap();
        hourly_forecast[i+1].hour /= 100;
    }

    // icon
    let child_selector_str = "#hourly_forecast .pictos-1h img";
    let child_selector = scraper::Selector::parse(child_selector_str).unwrap();
    let children = document.select(&child_selector);
    for (i, child) in children.enumerate() {
        let icon = child.attr("src").map(str::to_owned).unwrap();
        let title = child.attr("title").map(str::to_owned).unwrap();
        hourly_forecast[i].icon = (icon, title);
    }

    // temperature
    let child_selector_str = "#hourly_forecast .temps td span";
    let child_selector = scraper::Selector::parse(child_selector_str).unwrap();
    let children = document.select(&child_selector);
    for (i, child) in children.enumerate() {
        let temperature_text_raw = child.text().collect::<String>();
        let temperature_str = temperature_text_raw.trim();
        let digits = temperature_str.chars().filter(|c| c.is_digit(10)).collect::<String>();
        hourly_forecast[i].temperature = digits.parse().unwrap();
    }

    // windchill
    let child_selector_str = "#hourly_forecast .temperature-felt td";
    let child_selector = scraper::Selector::parse(child_selector_str).unwrap();
    let children = document.select(&child_selector);
    for (i, child) in children.enumerate() {
        let text_raw = child.text().collect::<String>();
        let text_trimmed_str = text_raw.trim();
        let digits = text_trimmed_str.chars().filter(|c| c.is_digit(10)).collect::<String>();
        hourly_forecast[i].windchill = digits.parse().unwrap();
    }

    // wind dir
    let child_selector_str = ".windspeeds .glyph.winddir";
    let child_selector = scraper::Selector::parse(child_selector_str).unwrap();
    let children = document.select(&child_selector);
    for (i, child) in children.enumerate() {
        hourly_forecast[i].winddir = child.text().collect::<String>().trim().parse().unwrap();
    }

    // wind speed
    let child_selector_str = "#hourly_forecast .windspeed td";
    let child_selector = scraper::Selector::parse(child_selector_str).unwrap();
    let children = document.select(&child_selector);
    for (i, child) in children.enumerate() {
        hourly_forecast[i].windspeeds.0 = child.text().collect::<String>().trim().parse().unwrap();
    }
    let child_selector_str = "#hourly_forecast .windgust td";
    let child_selector = scraper::Selector::parse(child_selector_str).unwrap();
    let children = document.select(&child_selector);
    for (i, child) in children.enumerate() {
        hourly_forecast[i].windspeeds.1 = child.text().collect::<String>().trim().parse().unwrap();
    }

    // precipitation probability
    let child_selector_str = "#hourly_forecast .precip-prop td span";
    let child_selector = scraper::Selector::parse(child_selector_str).unwrap();
    let children = document.select(&child_selector);
    for (i, child) in children.enumerate() {
        let text_raw = child.text().collect::<String>();
        let text_trimmed_str = text_raw.trim();
        let digits = text_trimmed_str.chars().filter(|c| c.is_digit(10)).collect::<String>();
        hourly_forecast[i].precip_prob = digits.parse().unwrap();
    }

    // precipitation in millimiters
    let child_selector_str = "#hourly_forecast .precip td span";
    let child_selector = scraper::Selector::parse(child_selector_str).unwrap();
    let children = document.select(&child_selector);
    for (i, child) in children.enumerate() {
        let text_raw = child.text().collect::<String>();
        let text_trimmed_str = text_raw.trim();

        if text_trimmed_str.len() > 0 {
            hourly_forecast[i].precip_mm = Some(text_trimmed_str.parse().unwrap());
        }
    }

    // night ?
    let child_selector_str = ".picto.hourly-view .icons .cell .pictoicon";
    let child_selector = scraper::Selector::parse(child_selector_str).unwrap();
    let children = document.select(&child_selector);
    for (i, child) in children.enumerate() {
        let classes = child.attr("class").map(|c| c.split_whitespace().collect::<Vec<_>>()).unwrap();
        let night_class_found = classes.iter().find(|c| c.contains("night")).is_some();
        hourly_forecast[i].night = night_class_found;
    }

    println!("{}", serde_json::to_string_pretty(&hourly_forecast).unwrap());

    Ok(())
}
