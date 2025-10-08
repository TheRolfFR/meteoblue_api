use std::{fs::File, io::Write, path::Path};

use reqwest::{cookie::Jar, Url};
use scraper::{ElementRef, Html};
use serde_derive::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct HourlyForecast {
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

#[derive(Debug, Serialize)]
pub struct CurrentWeather {
    header: String,
    city: String,
    icon: (String, String), // (Weather icon URL, title)
    temperature: i8, // +-127° should be enough
    description: String
}

#[derive(Debug, Serialize)]
pub struct ForecastPage {
    current_weather: CurrentWeather,
    hourly_forecast: [HourlyForecast; 24]
}

pub fn load_from_url(url: &str) -> Option<String> {

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

fn first_child<'a>(html: &'a Html, selector: &str) -> Option<ElementRef<'a>> {
    let selector = scraper::Selector::parse(selector).ok()?;

    let mut select = html.select(&selector);
    let el_ref = select.next();
    el_ref
}

fn extract_text(html: &Html, selector: &str) -> Option<String> {
    let first_child = first_child(html, selector)?;
    Some(first_child.text().collect())
}

fn get_current_weather_from_doc(html: &Html) -> CurrentWeather {
    let opt_header_text = extract_text(html, ".main-heading")
    .map(|header_text| header_text.trim().to_string());

    let opt_icon = first_child(html, ".current-picto > img")
    .map(|img| (
        img.attr("src").map(str::to_owned).unwrap_or_default(),
        img.attr("title").map(str::to_owned).unwrap_or_default(),
    ));

    let opt_city = first_child(html, ".current-heading h1")
    .and_then(|el| el.attr("content").map(str::to_owned));

    let opt_temperature = extract_text(html, ".current-temp")
    .map(|header_text| header_text.chars().filter(|c| c == &'-' || c.is_digit(10)).collect::<String>())
    .and_then(|temp_string| temp_string.parse().ok());

    let opt_description = extract_text(html, ".current-description > span:nth-child(1)")
    .map(|header_text| header_text.trim().to_string());

    CurrentWeather {
        header: opt_header_text.unwrap_or_default(),
        city: opt_city.unwrap_or_default(),
        icon: opt_icon.unwrap_or_default(),
        temperature: opt_temperature.unwrap_or_default(),
        description: opt_description.unwrap_or_default(),
    }
}


pub fn save_to_file<T: AsRef<Path>>(file_path: T, content: &str) -> std::io::Result<()> {
    let mut file = File::create(file_path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}


pub fn get_forecast_from_html(html_content: &str) -> ForecastPage {
    #[cfg(debug_assertions)]
    {
        save_to_file("content.html", html_content).unwrap();
    }

    let document = scraper::Html::parse_document(html_content);
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
        let digits = temperature_str.chars().filter(|c| c == &'-' || c.is_digit(10)).collect::<String>();
        hourly_forecast[i].temperature = digits.parse().unwrap();
    }

    // windchill
    let child_selector_str = "#hourly_forecast .temperature-felt td";
    let child_selector = scraper::Selector::parse(child_selector_str).unwrap();
    let children = document.select(&child_selector);
    for (i, child) in children.enumerate() {
        let text_raw = child.text().collect::<String>();
        let text_trimmed_str = text_raw.trim();
        let digits = text_trimmed_str.chars().filter(|c| c == &'-' || c.is_digit(10)).collect::<String>();
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

    let current_weather = get_current_weather_from_doc(&document);

    ForecastPage {
        current_weather,
        hourly_forecast,
    }
}

pub fn get_forecast_from_url(url: &str) -> ForecastPage {
    let html_content = load_from_url(url).unwrap();
    get_forecast_from_html(&html_content)
}
