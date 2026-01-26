use chrono::{DateTime, NaiveDate, Utc};
use std::fs;
use std::path::Path;

use super::item::{Person, PersonType};

#[derive(Debug, Default)]
pub struct NfoMetadata {
    pub title: Option<String>,
    pub original_title: Option<String>,
    pub sort_title: Option<String>,
    pub plot: Option<String>,
    pub tagline: Option<String>,
    pub rating: Option<f64>,
    pub mpaa: Option<String>,
    pub year: Option<i32>,
    pub runtime: Option<String>,
    pub premiered: Option<DateTime<Utc>>,
    pub genres: Vec<String>,
    pub studios: Vec<String>,
    pub people: Vec<Person>,
}

pub fn parse_nfo_file(path: &Path) -> Option<NfoMetadata> {
    let content = fs::read_to_string(path).ok()?;
    parse_nfo_content(&content)
}

pub fn parse_nfo_content(content: &str) -> Option<NfoMetadata> {
    let mut metadata = NfoMetadata::default();
    
    metadata.title = extract_tag(content, "title");
    metadata.original_title = extract_tag(content, "originaltitle");
    metadata.sort_title = extract_tag(content, "sorttitle");
    metadata.plot = extract_tag(content, "plot").or_else(|| extract_tag(content, "overview"));
    metadata.tagline = extract_tag(content, "tagline");
    metadata.mpaa = extract_tag(content, "mpaa");
    metadata.runtime = extract_tag(content, "runtime");
    
    if metadata.runtime.is_none() {
        // Try to find duration in fileinfo/streamdetails/video
        // Structure: <fileinfo><streamdetails><video><duration>...</duration></video></streamdetails></fileinfo>
        // extract_tag finds the first occurrence, which should work for the main video stream
        if let Some(fileinfo) = extract_tag(content, "fileinfo") {
             if let Some(streamdetails) = extract_tag(&fileinfo, "streamdetails") {
                 if let Some(video) = extract_tag(&streamdetails, "video") {
                     if let Some(duration) = extract_tag(&video, "duration") {
                         // Duration is usually in minutes (float)
                         if let Ok(mins) = duration.parse::<f64>() {
                             metadata.runtime = Some((mins.round() as i64).to_string());
                         }
                     }
                     
                     if metadata.runtime.is_none() {
                         if let Some(seconds) = extract_tag(&video, "durationinseconds") {
                             if let Ok(secs) = seconds.parse::<f64>() {
                                 let mins = (secs / 60.0).round() as i64;
                                 metadata.runtime = Some(mins.to_string());
                             }
                         }
                     }
                 }
             }
        }
    }
    
    if let Some(rating_str) = extract_tag(content, "rating") {
        metadata.rating = rating_str.parse::<f64>().ok();
    }
    
    if let Some(year_str) = extract_tag(content, "year") {
        metadata.year = year_str.parse::<i32>().ok();
    }
    
    if let Some(premiered_str) = extract_tag(content, "premiered").or_else(|| extract_tag(content, "aired")) {
        if let Ok(date) = NaiveDate::parse_from_str(&premiered_str, "%Y-%m-%d") {
            metadata.premiered = Some(date.and_hms_opt(0, 0, 0)?.and_utc());
        }
    }
    
    metadata.genres = extract_all_tags(content, "genre");
    metadata.studios = extract_all_tags(content, "studio");
    
    for actor_block in extract_blocks(content, "actor") {
        if let Some(name) = extract_tag(&actor_block, "name") {
            let role = extract_tag(&actor_block, "role");
            metadata.people.push(Person {
                name,
                role,
                person_type: PersonType::Actor,
            });
        }
    }
    
    for director in extract_all_tags(content, "director") {
        metadata.people.push(Person {
            name: director,
            role: None,
            person_type: PersonType::Director,
        });
    }
    
    for writer in extract_all_tags(content, "credits") {
        metadata.people.push(Person {
            name: writer,
            role: None,
            person_type: PersonType::Writer,
        });
    }
    
    Some(metadata)
}

fn extract_tag(content: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);
    
    let start = content.find(&start_tag)? + start_tag.len();
    let end = content[start..].find(&end_tag)? + start;
    
    let value = content[start..end].trim();
    if value.is_empty() {
        None
    } else {
        Some(decode_xml_entities(value))
    }
}

fn extract_all_tags(content: &str, tag: &str) -> Vec<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);
    let mut results = Vec::new();
    let mut search_from = 0;
    
    while let Some(start_pos) = content[search_from..].find(&start_tag) {
        let start = search_from + start_pos + start_tag.len();
        if let Some(end_pos) = content[start..].find(&end_tag) {
            let end = start + end_pos;
            let value = content[start..end].trim();
            if !value.is_empty() {
                results.push(decode_xml_entities(value));
            }
            search_from = end + end_tag.len();
        } else {
            break;
        }
    }
    
    results
}

fn extract_blocks(content: &str, tag: &str) -> Vec<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);
    let mut results = Vec::new();
    let mut search_from = 0;
    
    while let Some(start_pos) = content[search_from..].find(&start_tag) {
        let start = search_from + start_pos;
        if let Some(end_pos) = content[start..].find(&end_tag) {
            let end = start + end_pos + end_tag.len();
            results.push(content[start..end].to_string());
            search_from = end;
        } else {
            break;
        }
    }
    
    results
}

fn decode_xml_entities(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_movie_nfo() {
        let nfo = r#"
            <movie>
                <title>Test Movie</title>
                <originaltitle>Original Title</originaltitle>
                <plot>This is a test plot.</plot>
                <rating>8.5</rating>
                <year>2023</year>
                <genre>Action</genre>
                <genre>Drama</genre>
                <studio>Test Studio</studio>
                <director>John Doe</director>
            </movie>
        "#;
        
        let metadata = parse_nfo_content(nfo).unwrap();
        assert_eq!(metadata.title, Some("Test Movie".to_string()));
        assert_eq!(metadata.original_title, Some("Original Title".to_string()));
        assert_eq!(metadata.rating, Some(8.5));
        assert_eq!(metadata.year, Some(2023));
        assert_eq!(metadata.genres.len(), 2);
        assert_eq!(metadata.studios.len(), 1);
    }
}
