use crate::jellyfin::types::BaseItemDto;
use crate::util::QueryParams;

/// Apply filtering to a single item based on query parameters.
/// Returns true if the item should be included, false if it should be filtered out.
pub fn apply_item_filter(item: &BaseItemDto, params: &QueryParams) -> bool {
    // Media type filtering - includeItemTypes
    if let Some(include_types) = params.get("includeItemTypes") {
        let mut keep_item = false;
        for type_entry in include_types.split(',') {
            let type_entry = type_entry.trim();
            if (type_entry.eq_ignore_ascii_case("Movie") && item.item_type == "Movie")
                || (type_entry.eq_ignore_ascii_case("Series") && item.item_type == "Series")
                || (type_entry.eq_ignore_ascii_case("Season") && item.item_type == "Season")
                || (type_entry.eq_ignore_ascii_case("Episode") && item.item_type == "Episode")
            {
                keep_item = true;
                break;
            }
        }
        if !keep_item {
            return false;
        }
    }

    // Media type filtering - excludeItemTypes
    if let Some(exclude_types) = params.get("excludeItemTypes") {
        for type_entry in exclude_types.split(',') {
            let type_entry = type_entry.trim();
            if (type_entry.eq_ignore_ascii_case("Movie") && item.item_type == "Movie")
                || (type_entry.eq_ignore_ascii_case("Series") && item.item_type == "Series")
                || (type_entry.eq_ignore_ascii_case("Season") && item.item_type == "Season")
                || (type_entry.eq_ignore_ascii_case("Episode") && item.item_type == "Episode")
            {
                return false;
            }
        }
    }

    // ID filtering - ids
    if let Some(ids) = params.get("ids") {
        let mut keep_item = false;
        for id in ids.split(',') {
            if item.id == id.trim() {
                keep_item = true;
                break;
            }
        }
        if !keep_item {
            return false;
        }
    }

    // ID filtering - excludeItemIds
    if let Some(exclude_ids) = params.get("excludeItemIds") {
        for id in exclude_ids.split(',') {
            if item.id == id.trim() {
                return false;
            }
        }
    }

    // Genre filtering - genreIds
    if let Some(genre_ids) = params.get("genreIds") {
        let mut keep_item = false;
        if let Some(ref genre_items) = item.genre_items {
            for genre_id in genre_ids.split('|') {
                for genre_item in genre_items {
                    if genre_item.id == genre_id.trim() {
                        keep_item = true;
                        break;
                    }
                }
                if keep_item {
                    break;
                }
            }
        }
        if !keep_item {
            return false;
        }
    }

    // Genre filtering - genres (by name)
    if let Some(genres) = params.get("genres") {
        let mut keep_item = false;
        if let Some(ref item_genres) = item.genres {
            for genre in genres.split('|') {
                if item_genres.contains(&genre.trim().to_string()) {
                    keep_item = true;
                    break;
                }
            }
        }
        if !keep_item {
            return false;
        }
    }

    // Studio filtering - studioIds
    if let Some(studio_ids) = params.get("studioIds") {
        let mut keep_item = false;
        if let Some(ref studios) = item.studios {
            for studio_id in studio_ids.split('|') {
                for studio in studios {
                    if studio.id == studio_id.trim() {
                        keep_item = true;
                        break;
                    }
                }
                if keep_item {
                    break;
                }
            }
        }
        if !keep_item {
            return false;
        }
    }

    // Studio filtering - studios (by name)
    if let Some(studio_names) = params.get("studios") {
        let mut keep_item = false;
        if let Some(ref studios) = item.studios {
            for studio_name in studio_names.split('|') {
                for studio in studios {
                    if studio.name == studio_name.trim() {
                        keep_item = true;
                        break;
                    }
                }
                if keep_item {
                    break;
                }
            }
        }
        if !keep_item {
            return false;
        }
    }

    // Hierarchy filtering - seriesId
    if let Some(series_id) = params.get("seriesId") {
        match &item.series_id {
            Some(id) if id == series_id => {},
            _ => return false,
        }
    }

    // Hierarchy filtering - seasonId
    if let Some(season_id) = params.get("seasonId") {
        match &item.season_id {
            Some(id) if id == season_id => {},
            _ => return false,
        }
    }

    // Hierarchy filtering - parentIndexNumber
    if let Some(parent_index_str) = params.get("parentIndexNumber") {
        if let Ok(parent_index) = parent_index_str.parse::<i32>() {
            if item.parent_index_number != Some(parent_index) {
                return false;
            }
        }
    }

    // Hierarchy filtering - indexNumber
    if let Some(index_str) = params.get("indexNumber") {
        if let Ok(index) = index_str.parse::<i32>() {
            if item.index_number != Some(index) {
                return false;
            }
        }
    }

    // Name filtering - nameStartsWith
    if let Some(prefix) = params.get("nameStartsWith") {
        let item_name = item.sort_name.as_ref().unwrap_or(&item.name);
        if !item_name.to_lowercase().starts_with(&prefix.to_lowercase()) {
            return false;
        }
    }

    // Name filtering - nameStartsWithOrGreater
    if let Some(name_min) = params.get("nameStartsWithOrGreater") {
        let item_name = item.sort_name.as_ref().unwrap_or(&item.name);
        if item_name.to_lowercase() < name_min.to_lowercase() {
            return false;
        }
    }

    // Name filtering - nameLessThan
    if let Some(name_max) = params.get("nameLessThan") {
        let item_name = item.sort_name.as_ref().unwrap_or(&item.name);
        if item_name.to_lowercase() > name_max.to_lowercase() {
            return false;
        }
    }

    // Official rating filtering
    if let Some(ratings) = params.get("officialRatings") {
        let mut keep_item = false;
        if let Some(ref item_rating) = item.official_rating {
            for rating in ratings.split('|') {
                if item_rating == rating.trim() {
                    keep_item = true;
                    break;
                }
            }
        }
        if !keep_item {
            return false;
        }
    }

    // Rating filtering - minCommunityRating
    if let Some(min_rating_str) = params.get("minCommunityRating") {
        if let Ok(min_rating) = min_rating_str.parse::<f32>() {
            if item.community_rating.unwrap_or(0.0) < min_rating {
                return false;
            }
        }
    }

    // Date filtering - minPremiereDate
    if let Some(min_date_str) = params.get("minPremiereDate") {
        if let Ok(min_date) = chrono::DateTime::parse_from_rfc3339(min_date_str) {
            if let Some(ref premiere_date_str) = item.premiere_date {
                if let Ok(premiere_date) = chrono::DateTime::parse_from_rfc3339(premiere_date_str) {
                    if premiere_date < min_date {
                        return false;
                    }
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
    }

    // Date filtering - maxPremiereDate
    if let Some(max_date_str) = params.get("maxPremiereDate") {
        if let Ok(max_date) = chrono::DateTime::parse_from_rfc3339(max_date_str) {
            if let Some(ref premiere_date_str) = item.premiere_date {
                if let Ok(premiere_date) = chrono::DateTime::parse_from_rfc3339(premiere_date_str) {
                    if premiere_date > max_date {
                        return false;
                    }
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
    }

    // Year filtering - years
    if let Some(years) = params.get("years") {
        let mut keep_item = false;
        for year_str in years.split(',') {
            if let Ok(year) = year_str.trim().parse::<i32>() {
                if item.production_year == Some(year) {
                    keep_item = true;
                    break;
                }
            }
        }
        if !keep_item {
            return false;
        }
    }

    // User state filtering - isPlayed
    if let Some(is_played_str) = params.get("isPlayed") {
        if let Some(ref user_data) = item.user_data {
            let is_played = is_played_str.eq_ignore_ascii_case("true");
            if user_data.played != is_played {
                return false;
            }
        } else if is_played_str.eq_ignore_ascii_case("true") {
            return false;
        }
    }

    // User state filtering - isFavorite
    if let Some(is_favorite_str) = params.get("isFavorite") {
        if let Some(ref user_data) = item.user_data {
            let is_favorite = is_favorite_str.eq_ignore_ascii_case("true");
            if user_data.is_favorite != is_favorite {
                return false;
            }
        } else if is_favorite_str.eq_ignore_ascii_case("true") {
            return false;
        }
    }

    // Generic filters parameter
    if let Some(filters) = params.get("filters") {
        for filter in filters.split(',') {
            let filter = filter.trim();
            if filter.eq_ignore_ascii_case("IsFavorite")
                || filter.eq_ignore_ascii_case("IsFavoriteOrLikes")
            {
                if let Some(ref user_data) = item.user_data {
                    if !user_data.is_favorite {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
    }

    // If no filters matched to exclude, keep the item
    true
}

/// Apply filtering to a list of items
pub fn apply_items_filter(items: Vec<BaseItemDto>, params: &QueryParams) -> Vec<BaseItemDto> {
    items
        .into_iter()
        .filter(|item| apply_item_filter(item, params))
        .collect()
}
