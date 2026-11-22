use log::{debug, info};

const GITHUB_API_URL: &str =
    "https://api.github.com/repos/aarol/headset-battery-indicator/releases/latest";

#[derive(serde_derive::Deserialize)]
struct GithubRelease {
    tag_name: String,
}

pub fn check_for_updates(current_version: &str) -> Result<bool, Box<dyn std::error::Error>> {
    debug!("Checking for updates...");

    let response: GithubRelease = ureq::get(GITHUB_API_URL)
        .header("User-Agent", "headset-battery-indicator")
        .header("Accept", "application/vnd.github.v3+json")
        .call()?
        .body_mut()
        .read_json()?;

    let latest_version = response.tag_name.trim_start_matches('v');

    if is_newer_version(latest_version, current_version) {
        info!("New version available: {latest_version} (current: {current_version})");
        Ok(true)
    } else {
        info!("Using the latest version: {current_version}");
        Ok(false)
    }
}

/// Compares two semver-like version strings (e.g., "3.3.0" vs "3.2.1").
/// Returns true if `latest` is newer than `current`.
fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse_version =
        |v: &str| -> Vec<u32> { v.split('.').filter_map(|s| s.parse::<u32>().ok()).collect() };

    let latest_parts = parse_version(latest);
    let current_parts = parse_version(current);

    for (l, c) in latest_parts.iter().zip(current_parts.iter()) {
        match l.cmp(c) {
            std::cmp::Ordering::Greater => return true,
            std::cmp::Ordering::Less => return false,
            std::cmp::Ordering::Equal => continue,
        }
    }

    // If all compared parts are equal, check if latest has more parts
    latest_parts.len() > current_parts.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("3.4.0", "3.3.0"));
        assert!(is_newer_version("4.0.0", "3.9.9"));
        assert!(is_newer_version("3.3.1", "3.3.0"));
        assert!(!is_newer_version("3.3.0", "3.3.0"));
        assert!(!is_newer_version("3.2.0", "3.3.0"));
        assert!(!is_newer_version("2.9.9", "3.0.0"));
    }
}
