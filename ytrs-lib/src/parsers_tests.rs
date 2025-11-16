use super::parsers::*;
use serde_json::json;

#[test]
fn test_parse_badges_premium() {
    // Test video with Premium badge (style field is what matters)
    let video = json!({
        "badges": [
            {
                "metadataBadgeRenderer": {
                    "label": "Premium",
                    "style": "BADGE_STYLE_TYPE_MEMBERS_ONLY"
                }
            }
        ]
    });

    let (is_premium, badges) = parse_badges(&video);
    assert_eq!(is_premium, Some(true));
    assert_eq!(badges, Some(vec!["Premium".to_string()]));
}

#[test]
fn test_parse_badges_members_only() {
    // Test video with Members only badge (style field is what matters)
    let video = json!({
        "badges": [
            {
                "metadataBadgeRenderer": {
                    "label": "Members only",
                    "style": "BADGE_STYLE_TYPE_MEMBERS_ONLY"
                }
            }
        ]
    });

    let (is_premium, badges) = parse_badges(&video);
    assert_eq!(is_premium, Some(true));
    assert_eq!(badges, Some(vec!["Members only".to_string()]));
}

#[test]
fn test_parse_badges_members_only_style() {
    // Test video with Members only badge using style field (more reliable)
    let video = json!({
        "badges": [
            {
                "metadataBadgeRenderer": {
                    "label": "Só para membros",  // Portuguese example
                    "style": "BADGE_STYLE_TYPE_MEMBERS_ONLY"
                }
            }
        ]
    });

    let (is_premium, badges) = parse_badges(&video);
    assert_eq!(is_premium, Some(true));
    assert_eq!(badges, Some(vec!["Só para membros".to_string()]));
}

#[test]
fn test_parse_badges_top_standalone() {
    // Test video with topStandaloneBadge (alternative premium indicator)
    let video = json!({
        "topStandaloneBadge": {
            "metadataBadgeRenderer": {
                "label": "Members only"
            }
        }
    });

    let (is_premium, badges) = parse_badges(&video);
    assert_eq!(is_premium, Some(true));
    assert_eq!(badges, None); // topStandaloneBadge doesn't add to badges list
}

#[test]
fn test_parse_badges_normal_video() {
    // Test normal video without premium badges
    let video = json!({
        "videoId": "dQw4w9WgXcQ",
        "title": {
            "simpleText": "Test Video"
        }
    });

    let (is_premium, badges) = parse_badges(&video);
    assert_eq!(is_premium, None);
    assert_eq!(badges, None);
}

#[test]
fn test_parse_badges_multiple_badges() {
    // Test video with multiple badges including Premium
    let video = json!({
        "badges": [
            {
                "metadataBadgeRenderer": {
                    "label": "4K"
                }
            },
            {
                "metadataBadgeRenderer": {
                    "label": "Premium",
                    "style": "BADGE_STYLE_TYPE_MEMBERS_ONLY"
                }
            },
            {
                "metadataBadgeRenderer": {
                    "label": "CC"
                }
            }
        ]
    });

    let (is_premium, badges) = parse_badges(&video);
    assert_eq!(is_premium, Some(true));
    assert_eq!(
        badges,
        Some(vec![
            "4K".to_string(),
            "Premium".to_string(),
            "CC".to_string()
        ])
    );
}

#[test]
fn test_parse_video_renderer_with_premium() {
    // Test parsing a complete video renderer with premium badge
    let video = json!({
        "videoId": "test123",
        "title": {
            "simpleText": "Premium Test Video"
        },
        "badges": [
            {
                "metadataBadgeRenderer": {
                    "label": "Premium",
                    "style": "BADGE_STYLE_TYPE_MEMBERS_ONLY"
                }
            }
        ],
        "thumbnail": {
            "thumbnails": [
                {
                    "url": "https://i.ytimg.com/vi/test123/default.jpg",
                    "width": 120,
                    "height": 90
                }
            ]
        }
    });

    let result = parse_video_renderer(&video).unwrap();
    assert_eq!(result.video_id, Some("test123".to_string()));
    assert_eq!(result.title, "Premium Test Video");
    assert_eq!(result.is_premium, Some(true));
    assert_eq!(result.badges, Some(vec!["Premium".to_string()]));
}
