# Checkpoint (2026-09-21) — Turn 16: review; NEXT: user review

## Done — 1 commit (green: 83 tests, clippy + fmt clean)

- **`a8726cb` parser hygiene**: `join_notes`/`take_stroke` own `String`s
  (4× `Box::leak` gone — was per-parse intentional leaks to fit `&str`
  lifetimes) + `parse_target_text` true last-wins (winner's source word
  not echoed in notes; beaten word kept). No behavior change besides
  winner-word notes; all 33 parser tests pass unmodified.

## Review findings (read: all spec docs, code, 83 tests; ran clippy/fmt)

- Codebase is coherent: IDL ↔ parser ↔ encoder ↔ scraper match their
  docs (`idl.md`, `run-bike-notation.md`, `higdon.md`, `myswimpro.md`,
  `swimdojo-site.md`, `rustyfit.md`); spec Generator 1–7 + Scraper 1–3
  all closed (tasks.md).
- Debt paid above: `Box::leak` plumbing. Remaining `Box::leak`: none.
- Known bugs: none found. `zwo` two-pass text scan + `tag_text`
  substring matching is O(n²)-ish but inputs are KB-scale; fine.
- Minor notes (not fixed, by design): `zwo` ignores `<textevent>` cues;
  `parse_target_text` `@`-re-split means a literal `@` in prose
  splits; `Rest`/`Cross` day cells parse to notes-only zero-step bodies.

## Deferred (unchanged from turn 15)

- Named race pace (`5K pace` → needs pace map), ramp expansion (steady
  midpoint today), `FreeRide` open steps (target None — correct).
- `.zwo` `<textevent>` cues dropped (notes only carry labels today).
