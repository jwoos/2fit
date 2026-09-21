//! Tests for the Higdon grid scraper (offline; saved Novice 1 HTML).

use super::*;

const NOVICE1: &str = include_str!("../../tests/data/higdon-novice1.html");

#[test]
fn page_title_and_links() {
    assert!(Higdon::page_title(NOVICE1).contains("Novice 1"));
    let links = Higdon::parse_program_links(NOVICE1);
    // 14 program links on Novice 1 (marathon groups + cross-links).
    assert!(links.len() >= 10, "links: {}", links.len());
    assert!(
        links.iter().any(|(_, u)| u.contains("novice-1-marathon")),
        "{links:?}"
    );
}

#[test]
fn plan_tables_shape() {
    let tables = Higdon::parse_plan_tables(NOVICE1);
    assert_eq!(tables.len(), 2, "miles + km tables");
    assert_eq!(tables[0].0, "mi");
    assert_eq!(tables[1].0, "km");
    assert_eq!(tables[0].1.len(), 18, "18 weeks");
    // Week 1 miles row: Rest | 3 mi run ×3 | Rest | 6 | Cross.
    let (_, weeks) = &tables[0];
    let (no, cells) = &weeks[0];
    assert_eq!(no, "1");
    assert_eq!(
        cells,
        &vec![
            "Rest".to_owned(),
            "3 mi run".to_owned(),
            "3 mi run".to_owned(),
            "3 mi run".to_owned(),
            "Rest".to_owned(),
            "6".to_owned(),
            "Cross".to_owned(),
        ]
    );
    // Week 8 Saturday is Rest, Sunday the Half Marathon race marker.
    let (_, cells) = &weeks[7];
    assert_eq!(cells[5], "Rest");
    assert_eq!(cells[6], "Half Marathon");
    // Km view of week 1: same cells converted.
    let (_, kweeks) = &tables[1];
    assert_eq!(kweeks[0].1[1], "4.8 km run");
    assert_eq!(kweeks[0].1[5], "9.7");
}

#[test]
fn normalize_weeks_parses() {
    let tables = Higdon::parse_plan_tables(NOVICE1);
    let body = normalize_weeks(&tables);
    // Week header + bare cells + rest-day annotation (no day prefixes —
    // the parser has no day model).
    assert!(body.contains("Week 1\n"));
    assert!(body.contains("3 mi run\n"));
    assert!(body.contains("6 mi\n"));
    assert!(!body.contains("Tue:"), "{body:?}");
    assert!(body.contains("—>Rest days: Mon Rest, Fri Rest, Sun Cross\n"));
    assert!(body.contains("Half Marathon"));
    // The whole plan parses with the higdon_run schema.
    let schema = fit_generator::parser::schema::higdon_run();
    let w = fit_generator::parser::swimdojo::parse_with_schema(
        &body,
        fit_core::Pool::yards25(),
        None,
        &schema,
    )
    .expect("plan body parses");
    assert!(!w.flat_steps().is_empty());
    // Week 1: 3+3+3 mi + 6 mi = 4 distance steps (+ annotation).
    let week1: String = body
        .lines()
        .take_while(|l| !l.starts_with("Week 2"))
        .collect::<Vec<_>>()
        .join("\n");
    let w1 = fit_generator::parser::swimdojo::parse_with_schema(
        &week1,
        fit_core::Pool::yards25(),
        None,
        &schema,
    )
    .unwrap();
    assert_eq!(w1.flat_steps().len(), 4);
}

#[test]
fn plan_days_split_per_workout() {
    let tables = Higdon::parse_plan_tables(NOVICE1);
    let days = plan_days(&tables);
    // 18 weeks × 7 days = 126 cells, none empty on Novice 1.
    assert_eq!(days.len(), 126);
    assert_eq!(days[0], ("W1 Mon".to_owned(), "Rest".to_owned()));
    assert_eq!(days[1], ("W1 Tue".to_owned(), "3 mi run".to_owned()));
    assert_eq!(days[5], ("W1 Sat".to_owned(), "6 mi".to_owned()));
    // Race marker kept verbatim (caller decides: skip or annotate).
    assert!(
        days.iter()
            .any(|(d, c)| c == "Half Marathon" && d == "W8 Sun")
    );
    // Bodies parse: a work day → 1 step; a rest day → 0 steps.
    let schema = fit_generator::parser::schema::higdon_run();
    let parse = |body: &str| {
        fit_generator::parser::swimdojo::parse_with_schema(
            body,
            fit_core::Pool::yards25(),
            None,
            &schema,
        )
        .unwrap()
    };
    assert_eq!(parse(&day_body("W1 Tue", "3 mi run")).flat_steps().len(), 1);
    assert_eq!(parse(&day_body("W1 Sat", "6 mi")).flat_steps().len(), 1);
    assert_eq!(parse(&day_body("W1 Mon", "Rest")).flat_steps().len(), 0);
    assert_eq!(
        parse(&day_body("W8 Sun", "Half Marathon"))
            .flat_steps()
            .len(),
        0
    );
    // fetch_days titles carry plan + day.
    assert_eq!(day_body("W1 Tue", "3 mi run"), "3 mi run\n");
    assert_eq!(day_body("W1 Mon", "Rest"), "—>W1 Mon Rest\n");
}

#[test]
fn day_markers() {
    for m in [
        "Rest",
        "Cross",
        "Half Marathon",
        "Marathon",
        "5K Race",
        "5K Test",
        "10K Race",
    ] {
        assert!(is_day_marker(m), "{m}");
    }
    for s in ["3 mi run", "6", "30 min tempo run", "8 x 400 5K pace"] {
        assert!(!is_day_marker(s), "{s}");
    }
    assert_eq!(normalize_cell("6"), "6 mi");
    assert_eq!(normalize_cell("3 mi run"), "3 mi run");
}
