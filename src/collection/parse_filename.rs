use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq)]
pub struct EpisodeInfo {
    pub season: i32,
    pub episode: i32,
    pub end_episode: Option<i32>,
}

static EPISODE_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

fn get_patterns() -> &'static Vec<Regex> {
    EPISODE_PATTERNS.get_or_init(|| {
        vec![
            Regex::new(r"(?i)s(\d+)e(\d+)(?:e(\d+))?").unwrap(),
            Regex::new(r"(?i)(\d+)x(\d+)(?:x(\d+))?").unwrap(),
            Regex::new(r"(?i)season\s*(\d+).*episode\s*(\d+)").unwrap(),
            Regex::new(r"(?i)(\d{4})-(\d{2})-(\d{2})").unwrap(),
        ]
    })
}

pub fn parse_episode_from_filename(filename: &str) -> Option<EpisodeInfo> {
    let patterns = get_patterns();
    
    for pattern in patterns {
        if let Some(caps) = pattern.captures(filename) {
            if caps.get(0).unwrap().as_str().contains('-') {
                if let (Some(year), Some(month), Some(day)) = (
                    caps.get(1).and_then(|m| m.as_str().parse::<i32>().ok()),
                    caps.get(2).and_then(|m| m.as_str().parse::<i32>().ok()),
                    caps.get(3).and_then(|m| m.as_str().parse::<i32>().ok()),
                ) {
                    return Some(EpisodeInfo {
                        season: year,
                        episode: month * 100 + day,
                        end_episode: None,
                    });
                }
            } else {
                let season = caps.get(1)?.as_str().parse::<i32>().ok()?;
                let episode = caps.get(2)?.as_str().parse::<i32>().ok()?;
                let end_episode = caps.get(3).and_then(|m| m.as_str().parse::<i32>().ok());
                
                return Some(EpisodeInfo {
                    season,
                    episode,
                    end_episode,
                });
            }
        }
    }
    
    None
}

pub fn clean_title(filename: &str) -> String {
    let mut title = filename.to_string();
    
    if let Some(pos) = title.rfind('.') {
        title = title[..pos].to_string();
    }
    
    let patterns = get_patterns();
    for pattern in patterns {
        if let Some(m) = pattern.find(&title) {
            title = title[..m.start()].to_string();
            break;
        }
    }
    
    title = title.replace('_', " ");
    title = title.replace('.', " ");
    
    title.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_s01e04() {
        let info = parse_episode_from_filename("Show.Name.S01E04.mkv").unwrap();
        assert_eq!(info.season, 1);
        assert_eq!(info.episode, 4);
        assert_eq!(info.end_episode, None);
    }

    #[test]
    fn test_parse_s03e04e05() {
        let info = parse_episode_from_filename("Show.Name.S03E04E05.mkv").unwrap();
        assert_eq!(info.season, 3);
        assert_eq!(info.episode, 4);
        assert_eq!(info.end_episode, Some(5));
    }

    #[test]
    fn test_parse_3x08() {
        let info = parse_episode_from_filename("Show.Name.3x08.mkv").unwrap();
        assert_eq!(info.season, 3);
        assert_eq!(info.episode, 8);
    }

    #[test]
    fn test_parse_date() {
        let info = parse_episode_from_filename("Show.Name.2023-05-15.mkv").unwrap();
        assert_eq!(info.season, 2023);
        assert_eq!(info.episode, 515);
    }

    #[test]
    fn test_clean_title() {
        assert_eq!(
            clean_title("Show.Name.S01E04.1080p.mkv"),
            "Show Name"
        );
        assert_eq!(
            clean_title("Another_Show_3x08.mp4"),
            "Another Show"
        );
    }
}
