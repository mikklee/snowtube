//! Constants used throughout the library

use rand::Rng;

/// Base URL for InnerTube API
pub const INNERTUBE_API_BASE: &str = "https://www.youtube.com/youtubei/v1";

/// InnerTube API key (extracted from YouTube web client)
pub const INNERTUBE_API_KEY: &str = "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8";

/// InnerTube client name
pub const INNERTUBE_CLIENT_NAME: &str = "WEB";

/// InnerTube client version
pub const INNERTUBE_CLIENT_VERSION: &str = "2.20241030.01.00";

/// Generates a random user agent string to confuse tracking
pub fn random_user_agent() -> String {
    let mut rng = rand::thread_rng();

    // Randomly select browser type
    let browser_type = rng.gen_range(0..4);

    match browser_type {
        0 => generate_chrome_ua(&mut rng),
        1 => generate_firefox_ua(&mut rng),
        2 => generate_safari_ua(&mut rng),
        _ => generate_edge_ua(&mut rng),
    }
}

fn generate_chrome_ua(rng: &mut impl Rng) -> String {
    let os = random_os(rng);
    let chrome_version = rng.gen_range(120..131);
    let chrome_patch = rng.gen_range(0..10);

    format!(
        "Mozilla/5.0 ({}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{}.0.0.{} Safari/537.36",
        os, chrome_version, chrome_patch
    )
}

fn generate_firefox_ua(rng: &mut impl Rng) -> String {
    let firefox_version = rng.gen_range(125..132);
    let os_type = rng.gen_range(0..3);

    let os = match os_type {
        0 => format!("Windows NT 10.0; Win64; x64; rv:{}.0", firefox_version),
        1 => format!("Macintosh; Intel Mac OS X 10.15; rv:{}.0", firefox_version),
        _ => format!("X11; Linux x86_64; rv:{}.0", firefox_version),
    };

    format!(
        "Mozilla/5.0 ({}) Gecko/20100101 Firefox/{}.0",
        os, firefox_version
    )
}

fn generate_safari_ua(rng: &mut impl Rng) -> String {
    let safari_version = rng.gen_range(16..19);
    let macos_minor = rng.gen_range(0..8);

    format!(
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_{}) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/{}.0 Safari/605.1.15",
        macos_minor, safari_version
    )
}

fn generate_edge_ua(rng: &mut impl Rng) -> String {
    let os = random_os(rng);
    let edge_version = rng.gen_range(120..131);
    let edge_patch = rng.gen_range(0..10);

    format!(
        "Mozilla/5.0 ({}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{}.0.0.{} Safari/537.36 Edg/{}.0.0.0",
        os, edge_version, edge_patch, edge_version
    )
}

fn random_os(rng: &mut impl Rng) -> String {
    let os_type = rng.gen_range(0..3);

    match os_type {
        0 => "Windows NT 10.0; Win64; x64".to_string(),
        1 => {
            let macos_minor = rng.gen_range(0..8);
            format!("Macintosh; Intel Mac OS X 10_15_{}", macos_minor)
        }
        _ => "X11; Linux x86_64".to_string(),
    }
}
