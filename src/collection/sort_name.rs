use regex::Regex;

/// Generate a sort name from a display name, matching Go server behavior.
/// Strips leading articles, removes year suffix, lowercases.
pub fn make_sort_name(name: &str) -> String {
    // Start with lowercasing and trimming whitespace
    let mut title = name.trim().to_lowercase();
    
    // Remove leading articles
    for prefix in &["the ", "a ", "an "] {
        if title.starts_with(prefix) {
            title = title[prefix.len()..].trim_start().to_string();
            break;
        }
    }
    
    // Remove leading whitespace and punctuation
    title = title.trim_start_matches(|c: char| c.is_whitespace() || c.is_ascii_punctuation()).to_string();
    
    // Remove year suffix if present (e.g., " (2022)")
    title = remove_year_suffix(&title);
    
    title
}

fn remove_year_suffix(name: &str) -> String {
    // Match patterns like " (1999)" or " (2022)" at the end
    let re = Regex::new(r"\s*\(\d{4}\)\s*$").unwrap();
    re.replace(name, "").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_sort_name() {
        assert_eq!(make_sort_name("The Hunting Party"), "hunting party");
        assert_eq!(make_sort_name("A Beautiful Mind"), "beautiful mind");
        assert_eq!(make_sort_name("An Inconvenient Truth"), "inconvenient truth");
        assert_eq!(make_sort_name("Beauty (2022)"), "beauty");
        assert_eq!(make_sort_name("The Matrix (1999)"), "matrix");
        assert_eq!(make_sort_name("On Chesil Beach (2018)"), "on chesil beach");
    }
}
