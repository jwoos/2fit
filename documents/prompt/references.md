# References

- documents/idl.md — IDL spec + decisions (canonical; links from Plan above)
- documents/rustyfit.md — rustyfit 0.10.2 encoder API, verified from crate source
- documents/swimdojo-grammar.txt — swimdojo notation grammar (persisted; was `/tmp/sd-howto.txt`)
- documents/swimdojo-site.md — swimdojo site layout/filters for the scraper (detail URL + body-HTML block confirmed against 3 real pages, turn 5)
- documents/swimdojo-fixtures/{goblin-shark,box-crab,sea-otter}.txt — real workouts normalized to notation; parser regression fixtures (assert stated totals 1400/1000/800)
- swimdojo workouts: https://www.swimdojo.com/workouts (detail: `/workouts/YYYY/M/D/slug`; filters By Distance/Level/Stroke, `?tag=`, `?author=<id>`, `?offset=<epoch-ms>`). Body HTML: `div[data-layout-label="Post Body"]` → `.sqs-block.html-block` → div.sqs-html-content
- rustyfit (chosen): https://docs.rs/rustyfit — v0.10.2, encode+decode; WorkoutStep: https://docs.rs/rustyfit/latest/rustyfit/mesgdef/struct.WorkoutStep.html
- fit-sdk-rust (rejected): https://docs.rs/fit-sdk-rust — std crate named `fit`, encodes, younger.
