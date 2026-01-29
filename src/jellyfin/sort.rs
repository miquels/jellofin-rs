use crate::jellyfin::types::BaseItemDto;
use crate::util::QueryParams;

/// Apply sorting to a list of items based on query parameters
pub fn apply_item_sorting(mut items: Vec<BaseItemDto>, params: &QueryParams) -> Vec<BaseItemDto> {
    let sort_by = match params.get("sortBy") {
        Some(s) => s,
        None => return items, // No sorting requested
    };

    let sort_fields: Vec<&str> = sort_by.split(',').map(|s| s.trim()).collect();
    let sort_descending = params
        .get("sortOrder")
        .map(|s| s.eq_ignore_ascii_case("descending"))
        .unwrap_or(false);

    // Handle random sorting specially - just return items as-is for now
    // (true random would require rand crate)
    if sort_fields.iter().any(|f| f.eq_ignore_ascii_case("random")) {
        return items;
    }

    items.sort_by(|a, b| {
        for field in &sort_fields {
            let field_lower = field.to_lowercase();

            let ordering = match field_lower.as_str() {
                "communityrating" => {
                    let a_val = a.community_rating.unwrap_or(0.0);
                    let b_val = b.community_rating.unwrap_or(0.0);
                    a_val
                        .partial_cmp(&b_val)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }
                "datecreated" => {
                    // Parse date strings for comparison
                    let a_date = a
                        .date_created
                        .as_ref()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
                    let b_date = b
                        .date_created
                        .as_ref()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
                    a_date.cmp(&b_date)
                }
                "premieredate" => {
                    let a_date = a
                        .premiere_date
                        .as_ref()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
                    let b_date = b
                        .premiere_date
                        .as_ref()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
                    a_date.cmp(&b_date)
                }
                "productionyear" => a.production_year.cmp(&b.production_year),
                "sortname" => {
                    let a_name = a.sort_name.as_ref().unwrap_or(&a.name);
                    let b_name = b.sort_name.as_ref().unwrap_or(&b.name);
                    a_name.to_lowercase().cmp(&b_name.to_lowercase())
                }
                "name" => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                "runtime" => a.runtime_ticks.cmp(&b.runtime_ticks),
                "playcount" => {
                    let a_count = a.user_data.as_ref().map(|d| d.play_count).unwrap_or(0);
                    let b_count = b.user_data.as_ref().map(|d| d.play_count).unwrap_or(0);
                    a_count.cmp(&b_count)
                }
                "dateplayed" => {
                    let a_date = a
                        .user_data
                        .as_ref()
                        .and_then(|d| d.last_played_date.as_ref())
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
                    let b_date = b
                        .user_data
                        .as_ref()
                        .and_then(|d| d.last_played_date.as_ref())
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
                    a_date.cmp(&b_date)
                }
                "indexnumber" => a.index_number.cmp(&b.index_number),
                "parentindexnumber" => a.parent_index_number.cmp(&b.parent_index_number),
                _ => std::cmp::Ordering::Equal,
            };

            if ordering != std::cmp::Ordering::Equal {
                return if sort_descending {
                    ordering.reverse()
                } else {
                    ordering
                };
            }
        }
        std::cmp::Ordering::Equal
    });

    items
}
