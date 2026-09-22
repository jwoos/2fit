# Checkpoint (2026-09-22) — Turn 19: .zwo textevent cues; NEXT: user review

## Done — 1 commit (green: 88 tests, clippy + fmt clean)

- **`2d98273` textevent cues** (closes last `.zwo` deferral): flat
  event loop tracks step-nesting frames (Empty + Start/End; cue `Start`
  treated as attribute-carried); `read_cue()` (`message`/`mssage`,
  `timeoffset`/`TimeOffset` float secs, run `distoffset` meters);
  `attach_cues()` (owner = step holding `base+offset`; past-end → last
  step; bare message at offset 0, `@M:SS` deeper, `@Nm` verbatim for
  distance); `push_cue()` (deepest expansion holder; intervals phases
  re-clocked `@1:30`→`@0:30` since inner steps repeat). Found live bug
  in review: flat loop based cues on step *start* order, so mid-step
  offsets landed on the wrong step (`@6:30` instead of `@1:30`) —
  fixed with frames. Tests: `textevents_attach_as_cues` (step/offset/
  ramp/interval/`TextEvent`+`mssage`+`distoffset`/empty/back-compat —
  88 total). Live: cue smoke → 246 B `.fit`; SST cueless unchanged.
  Docs: `run-bike-notation.md` § `.zwo` coaching cues (h4l evidence).

## Deferred (remaining)

- `FreeRide` open steps (target None — correct by design).
