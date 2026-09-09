//! Offline tests: real feed/page snapshots in `tests/data/`.

use super::super::traits::ListFilter;
use super::{Swimdojo, detail_body, normalize_body};

const FEED: &str = include_str!("../../tests/data/feed.xml");
const GOBLIN_HTML: &str = include_str!("../../tests/data/raw-goblin-shark.html");
const BOX_HTML: &str = include_str!("../../tests/data/raw-box-crab.html");
const OTTER_HTML: &str = include_str!("../../tests/data/raw-sea-otter.html");
const HOURGLASS_HTML: &str = include_str!("../../tests/data/raw-hourglass-dolphin.html");
const PAGE: &str = include_str!("../../tests/data/detail-goblin-shark.html");

const GOBLIN_FIXTURE: &str = include_str!("../../../documents/swimdojo-fixtures/goblin-shark.txt");
const BOX_FIXTURE: &str = include_str!("../../../documents/swimdojo-fixtures/box-crab.txt");
const OTTER_FIXTURE: &str = include_str!("../../../documents/swimdojo-fixtures/sea-otter.txt");

fn normalized_eq(body: &str, fixture: &str) -> bool {
    body.replace('\u{2019}', "'") == fixture.replace('\u{2019}', "'")
}

#[test]
fn feed_items_parse() {
    let items = Swimdojo::parse_feed(FEED, usize::MAX).unwrap();
    assert_eq!(items.len(), 20);
    let g = &items[0];
    assert_eq!(g.title, "Goblin Shark");
    assert_eq!(
        g.url,
        "https://www.swimdojo.com/workouts/2025/5/19/goblin-shark"
    );
    assert_eq!(g.author.as_deref(), Some("meh"));
    assert!(
        g.description
            .as_deref()
            .unwrap_or_default()
            .contains("6,000")
    );
    // `pubDate: Thu, 14 Aug 2025 09:31:04 +0000` → epoch seconds.
    assert_eq!(g.published, Some(1_755_163_864));
}

#[test]
fn feed_url_params() {
    let f = ListFilter {
        tag: Some("IM".into()),
        author: Some("abc".into()),
        ..Default::default()
    };
    let url = Swimdojo::feed_url(&f, Some(123));
    assert!(url.contains("format=rss"), "{url}");
    assert!(url.contains("tag=IM"), "{url}");
    assert!(url.contains("author=abc"), "{url}");
    assert!(url.contains("offset=123"), "{url}");
}

#[test]
fn bodies_match_parser_fixtures() {
    // box-crab and sea-otter are byte-exact; goblin differs only in a
    // hand-normalized `'` vs the site's `'` (both parse identically).
    assert!(normalized_eq(&normalize_body(BOX_HTML), BOX_FIXTURE));
    assert!(normalized_eq(&normalize_body(OTTER_HTML), OTTER_FIXTURE));
    assert!(normalized_eq(&normalize_body(GOBLIN_HTML), GOBLIN_FIXTURE));
}

#[test]
fn open_water_body_keeps_lists() {
    let body = normalize_body(HOURGLASS_HTML);
    assert!(body.contains("Main set"), "{body}");
    assert!(body.contains("running start"), "{body}");
}

#[test]
fn detail_page_body_matches_feed() {
    let (title, html) = detail_body(PAGE).expect("post body block");
    assert_eq!(title, "Goblin Shark");
    assert!(normalized_eq(&normalize_body(&html), GOBLIN_FIXTURE));
}
