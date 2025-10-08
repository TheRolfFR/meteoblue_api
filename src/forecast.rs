use std::{fs::File, io::Write, path::Path};

use reqwest::{cookie::Jar, Url};
use scraper::{ElementRef, Html, Selector};
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
pub struct DayForecast {
    day_short: String,
    day_long: String,
    icon: (String, String),
    temperature: (i8, i8),
    wind: (u8, String),
    precip: Option<(u8, u8)>,
    sun: u8,
    precision: String
}

struct DaySelectors {
    day_short: Selector,
    day_long: Selector,
    icon: Selector,
    temperature_min: Selector,
    temperature_max: Selector,
    wind_dir: Selector,
    wind_speed: Selector,
    precip: Selector,
    sun: Selector,
    precision: Selector
}

#[derive(Debug, Serialize)]
pub struct ForecastPage {
    current_weather: CurrentWeather,
    day_forecast: Vec<DayForecast>,
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
    Some(first_child.text().collect::<String>().trim().to_owned())
}

fn extract_text_el<'a>(el: ElementRef<'a>, selector: &Selector) -> Option<String> {
    let mut select = el.select(selector);
    let first_child = select.next()?;
    Some(first_child.text().collect::<String>().trim().to_owned())
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

fn get_day_forecast_from_el<'a>(el: ElementRef<'a>, selectors: &DaySelectors) -> DayForecast {
    let day_short = extract_text_el(el, &selectors.day_short).unwrap_or_default();
    let day_long = extract_text_el(el, &selectors.day_long).unwrap_or_default();

    let icon =  el.select(&selectors.icon).next()
        .map(|img| (
            img.attr("src").map(str::to_owned).unwrap_or_default(),
            img.attr("title").map(str::to_owned).unwrap_or_default(),
        )).unwrap_or_default();

    let temp_max = extract_text_el(el, &selectors.temperature_max)
        .map(|header_text| header_text.chars().filter(|c| c == &'-' || c.is_digit(10)).collect::<String>())
        .and_then(|txt_str| txt_str.parse().ok()).unwrap_or_default();
    let temp_min = extract_text_el(el, &selectors.temperature_min)
        .map(|header_text| header_text.chars().filter(|c| c == &'-' || c.is_digit(10)).collect::<String>())
        .and_then(|txt_str| txt_str.parse().ok()).unwrap_or_default();

    let wind_dir = el.select(&selectors.wind_dir).next()
        .and_then(|found| found.attr("class")).map(|class| class.split_whitespace())
        .and_then(|words| words.last())
        .map(|res| res.to_owned()).unwrap_or_default();
    let wind_speed = extract_text_el(el, &selectors.wind_speed)
        .map(|header_text| header_text.chars().filter(|c| c.is_digit(10)).collect::<String>())
        .and_then(|txt_str| txt_str.parse().ok()).unwrap_or_default();

    let precip = extract_text_el(el, &selectors.precip)
        .and_then(|text| if "-" == &text { None } else { Some(text)})
        .map(|text| text.chars().filter(|c| c == &'-' || c.is_digit(10)).collect::<String>())
        .and_then(|range| range.split_once('-').map(|(a, b)| (a.parse().unwrap_or_default(), b.parse().unwrap_or_default()))
    );

    let sun = extract_text_el(el, &selectors.sun)
        .map(|header_text| header_text.chars().filter(|c| c.is_digit(10)).collect::<String>())
        .and_then(|txt_str| txt_str.parse().ok()).unwrap_or_default();

    let precision = el.select(&selectors.precision).next()
        .and_then(|found| found.attr("title"))
        .and_then(|title| title.split_once(':'))
        .map(|(_,b)| b.trim().to_owned()).unwrap_or_default();

    DayForecast { day_short,
        day_long,
        icon,
        temperature: (temp_min, temp_max),
        wind: (wind_speed, wind_dir),
        precip,
        sun,
        precision,
    }

}

fn get_day_forecast_from_doc(html: &Html) -> Vec<DayForecast> {
    let list_selector = scraper::Selector::parse("#tabs > .tab").unwrap();
    let tabs_tab_list = html.select(&list_selector);

    let day_selectors = DaySelectors {
        day_short: scraper::Selector::parse(".tab-day-short").unwrap(),
        day_long: scraper::Selector::parse(".tab-day-long").unwrap(),
        icon: scraper::Selector::parse(".weather-pictogram").unwrap(),
        temperature_min: scraper::Selector::parse(".temps > .tab-temp-min").unwrap(),
        temperature_max: scraper::Selector::parse(".temps > .tab-temp-max").unwrap(),
        wind_dir: scraper::Selector::parse(".wind .winddir").unwrap(),
        wind_speed: scraper::Selector::parse(".wind").unwrap(),
        precip: scraper::Selector::parse(".tab-precip").unwrap(),
        sun: scraper::Selector::parse(".tab-sun").unwrap(),
        precision: scraper::Selector::parse(".tab-predictability").unwrap(),
    };

    tabs_tab_list.map(|tab| {
        get_day_forecast_from_el(tab, &day_selectors)
    }).collect()
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

    let day_forecast = get_day_forecast_from_doc(&document);

    ForecastPage {
        current_weather,
        day_forecast,
        hourly_forecast,
    }
}

pub fn get_forecast_from_url(url: &str) -> ForecastPage {
    let html_content = load_from_url(url).unwrap();
    get_forecast_from_html(&html_content)
}
