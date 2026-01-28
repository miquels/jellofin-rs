use crate::jellyfin::types::BaseItemDto;
use crate::util::QueryParams;

/// Apply pagination to a list of items
/// Returns (paginated_items, start_index)
pub fn apply_pagination(
    items: Vec<BaseItemDto>,
    params: &QueryParams,
) -> (Vec<BaseItemDto>, usize) {
    let start_index = params
        .get("startIndex")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);

    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(100);

    let paginated = items
        .into_iter()
        .skip(start_index)
        .take(limit)
        .collect();

    (paginated, start_index)
}
