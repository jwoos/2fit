//! Offline tests: real WP API snapshot + article pages in `tests/data/`.

use super::{Myswimpro, normalize_body, normalize_step};

const POSTS: &str = include_str!("../../tests/data/msp-posts.json");
const PAGE_1800: &str = include_str!("../../tests/data/msp-1800-page.html");
const PAGE_COMEBACK: &str = include_str!("../../tests/data/msp-comeback-page.html");
const PAGE_TETHERED: &str = include_str!("../../tests/data/msp-tethered-page.html");

#[test]
fn posts_parse() {
    let items = Myswimpro::parse_posts(POSTS).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[1].title, "Try This 1,800m Swim Workout");
    assert_eq!(
        items[1].url,
        "https://blog.myswimpro.com/2022/06/09/try-this-1800m-swim-workout/"
    );
    assert!(items[1].published.is_some());
}

#[test]
fn body_normalizes_to_notation() {
    let json: serde_json::Value = serde_json::from_str(POSTS).unwrap();
    let html = json[1]["content"]["rendered"].as_str().unwrap();
    let body = normalize_body(html);
    // Section structure survives; spot-check the divergent vocabulary.
    for line in [
        "Warm Up",
        "Pre-Set (2x)",
        "Main Set (3x)",
        "Warm Down",
        "1 x 300 Freestyle @ 5:00 Every 3rd 25 Backstroke",
        "4 x 25s Freestyle Descend 1-4 @ :30 (w/ Snorkel)",
        "4 x 50s Freestyle Silent Swimming @ 1:00",
    ] {
        assert!(body.contains(line), "missing {line:?} in:\n{body}");
    }
    // Dryland cross-training is dropped, not parsed as swim.
    assert!(!body.contains("Pushups"), "{body}");
    std::fs::write("/tmp/msp-1800.txt", &body).unwrap();
}

#[test]
fn steps_normalize() {
    assert_eq!(normalize_step("4×50 Kick @ 1:10"), "4 x 50 Kick @ 1:10");
    assert_eq!(normalize_step("1×300 Freestyle"), "1 x 300 Freestyle");
    assert_eq!(normalize_step("1 x 25 Butterfly + 5 Pushups @ 0:50"), "");
}

#[test]
fn page_fixtures_normalize() {
    // 1800: full Warmup/Pre-Set/Main/Cool structure, dryland dropped.
    let b1800 = normalize_body(PAGE_1800);
    for line in [
        "Warm Up",
        "1 x 300 Freestyle @ 5:00 Every 3rd 25 Backstroke",
        "Pre-Set (2x)",
        "Main Set (3x)",
        "1 x 200 Freestyle @ 4:00 Negative Split",
        "Warm Down",
        "4 x 50s Freestyle Silent Swimming @ 1:00",
    ] {
        assert!(b1800.contains(line), "missing {line:?} in:\n{b1800}");
    }
    assert!(!b1800.contains("Pushups"), "{b1800}");

    // Comeback: meter-suffixed distances, extra Kick/IM sub-sections.
    let back = normalize_body(PAGE_COMEBACK);
    for line in [
        "Warm Up",
        "5x60m Freestyle, 15-30 seconds rest",
        "Kick & Drill",
        "1x300m Kick",
        "IM Set (2x)",
        "1x30m Butterfly",
        "Warm Down",
        "2x30m Freestyle",
    ] {
        assert!(back.contains(line), "missing {line:?} in:\n{back}");
    }

    // Tethered: time-based steps under "The Workout (30 seconds rest
    // between sets)". "Stroke Count"/"Time" example groups and the
    // `30 Minute Swim Tether Workout` h4 are not section headers, so the
    // body starts at `The Workout` (kept verbatim).
    let teth = normalize_body(PAGE_TETHERED);
    for line in [
        "The Workout (30 seconds rest between sets)",
        "1 x 3:00 Free Easy",
        "3 x 3:00 Free (With snorkel, 15 strokes hard, 15 strokes easy)",
    ] {
        assert!(teth.contains(line), "missing {line:?} in:\n{teth}");
    }
}
