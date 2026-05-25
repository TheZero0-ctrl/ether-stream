pub(super) fn parse_episode_progress_key(key: &str) -> (Option<i32>, Option<i32>) {
    let mut season = None;
    let mut episode = None;

    for part in key.split('|') {
        if let Some(raw) = part.strip_prefix("season:") {
            season = parse_key_number(raw);
        } else if let Some(raw) = part.strip_prefix("episode:") {
            episode = parse_key_number(raw);
        }
    }

    (season, episode)
}

pub(super) fn parse_key_number(raw: &str) -> Option<i32> {
    let trimmed = raw.trim();
    if trimmed == "None" {
        return None;
    }

    if let Some(inner) = trimmed.strip_prefix("Some(").and_then(|v| v.strip_suffix(')')) {
        return inner.trim().parse::<i32>().ok();
    }

    trimmed.parse::<i32>().ok()
}
