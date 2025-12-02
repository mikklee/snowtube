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

// ============================================================================
// Shorts detection tests (using US English locale: hl=en, gl=US)
// ============================================================================

#[test]
fn test_is_short_via_navigation_url() {
    // Test detecting Short via navigation endpoint URL containing /shorts/
    let video = json!({
        "videoId": "YdxFKyrxGfI",
        "title": {
            "simpleText": "Grading Test #shorts #memes"
        },
        "navigationEndpoint": {
            "commandMetadata": {
                "webCommandMetadata": {
                    "url": "/shorts/YdxFKyrxGfI"
                }
            }
        }
    });

    let result = is_short(&video);
    assert_eq!(result, Some(true));
}

#[test]
fn test_is_short_via_thumbnail_overlay() {
    // Test detecting Short via thumbnailOverlayTimeStatusRenderer with SHORTS style
    let video = json!({
        "videoId": "aEnz_yfqh1M",
        "title": {
            "simpleText": "Medical check up"
        },
        "thumbnailOverlays": [
            {
                "thumbnailOverlayTimeStatusRenderer": {
                    "style": "SHORTS",
                    "text": {
                        "simpleText": "SHORTS"
                    }
                }
            }
        ]
    });

    let result = is_short(&video);
    assert_eq!(result, Some(true));
}

#[test]
fn test_is_short_both_indicators() {
    // Test video with both navigation URL and thumbnail overlay indicating Short
    let video = json!({
        "videoId": "4Ttmglh12XE",
        "title": {
            "simpleText": "Hardest test in the world"
        },
        "navigationEndpoint": {
            "commandMetadata": {
                "webCommandMetadata": {
                    "url": "/shorts/4Ttmglh12XE"
                }
            }
        },
        "thumbnailOverlays": [
            {
                "thumbnailOverlayTimeStatusRenderer": {
                    "style": "SHORTS",
                    "text": {
                        "simpleText": "SHORTS"
                    }
                }
            }
        ]
    });

    let result = is_short(&video);
    assert_eq!(result, Some(true));
}

#[test]
fn test_is_not_short_regular_video() {
    // Test regular video with /watch?v= URL is not detected as Short
    let video = json!({
        "videoId": "CkX2KVhPOmM",
        "title": {
            "simpleText": "Regular Video Title"
        },
        "navigationEndpoint": {
            "commandMetadata": {
                "webCommandMetadata": {
                    "url": "/watch?v=CkX2KVhPOmM&pp=ygUEdGVzdA%3D%3D"
                }
            }
        },
        "thumbnailOverlays": [
            {
                "thumbnailOverlayTimeStatusRenderer": {
                    "style": "DEFAULT",
                    "text": {
                        "simpleText": "2:02:08"
                    }
                }
            }
        ]
    });

    let result = is_short(&video);
    assert_eq!(result, None);
}

#[test]
fn test_is_not_short_no_indicators() {
    // Test video without any navigation or overlay data
    let video = json!({
        "videoId": "test123",
        "title": {
            "simpleText": "Test Video"
        }
    });

    let result = is_short(&video);
    assert_eq!(result, None);
}

#[test]
fn test_parse_video_renderer_with_short() {
    // Test parsing a complete video renderer that is a Short
    let video = json!({
        "videoId": "YdxFKyrxGfI",
        "title": {
            "simpleText": "Grading Test #shorts #memes"
        },
        "navigationEndpoint": {
            "commandMetadata": {
                "webCommandMetadata": {
                    "url": "/shorts/YdxFKyrxGfI"
                }
            }
        },
        "thumbnailOverlays": [
            {
                "thumbnailOverlayTimeStatusRenderer": {
                    "style": "SHORTS",
                    "text": {
                        "simpleText": "SHORTS"
                    }
                }
            }
        ],
        "thumbnail": {
            "thumbnails": [
                {
                    "url": "https://i.ytimg.com/vi/YdxFKyrxGfI/default.jpg",
                    "width": 120,
                    "height": 90
                }
            ]
        }
    });

    let result = parse_video_renderer(&video).unwrap();
    assert_eq!(result.video_id, Some("YdxFKyrxGfI".to_string()));
    assert_eq!(result.title, "Grading Test #shorts #memes");
    assert_eq!(result.is_short, Some(true));
    assert_eq!(result.is_premium, None);
}

#[test]
fn test_parse_video_renderer_regular_video_not_short() {
    // Test parsing a regular video is not marked as Short
    let video = json!({
        "videoId": "CkX2KVhPOmM",
        "title": {
            "simpleText": "Regular Video"
        },
        "navigationEndpoint": {
            "commandMetadata": {
                "webCommandMetadata": {
                    "url": "/watch?v=CkX2KVhPOmM"
                }
            }
        },
        "thumbnailOverlays": [
            {
                "thumbnailOverlayTimeStatusRenderer": {
                    "style": "DEFAULT",
                    "text": {
                        "simpleText": "10:30"
                    }
                }
            }
        ],
        "thumbnail": {
            "thumbnails": [
                {
                    "url": "https://i.ytimg.com/vi/CkX2KVhPOmM/default.jpg",
                    "width": 120,
                    "height": 90
                }
            ]
        },
        "badges": [
            {
                "metadataBadgeRenderer": {
                    "label": "4K",
                    "style": "BADGE_STYLE_TYPE_SIMPLE"
                }
            }
        ]
    });

    let result = parse_video_renderer(&video).unwrap();
    assert_eq!(result.video_id, Some("CkX2KVhPOmM".to_string()));
    assert_eq!(result.is_short, None);
    assert_eq!(result.badges, Some(vec!["4K".to_string()]));
}
