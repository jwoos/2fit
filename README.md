# 2fit

Tool to convert written workouts to .fit files.

## Workspace

| crate | artifacts | purpose |
| --- | --- | --- |
| `core` | lib `fit_core` | format-independent workout domain model (the "IDL") |
| `generator` | lib `fit_generator`, bin `2fit-gen` | parses workout text → IDL → .fit |
| `scraper` | lib `fit_scraper`, bin `2fit-scrape` | scrapes written workouts from swim sites |

## Usage (generator, once implemented)

```sh
2fit-gen --file workout.txt -o workout.fit
cat workout.txt | 2fit-gen          # stdin → .fit on stdout
```

## Status

WIP — see `prompt.md` for plan and tasks.
