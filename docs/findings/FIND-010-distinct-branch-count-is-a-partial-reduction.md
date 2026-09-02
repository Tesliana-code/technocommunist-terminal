# FIND-010 — Distinct Entering-Branch Count Is a Partial, Not Sufficient, Economic Reduction

Status: OBSERVED
Protocol: PP-0.1-draft
Evidence: PATH-033

## Hypothesis under test (H4)

FIND-009 left the economic reduction of `StructuralConvergenceRegion`
(members + entries) to a scalar as an explicit open design decision, not
forced by missing structural information. PATH-033 attacks one specific
candidate: **economic structural importance depends primarily on the
number of DISTINCT branches entering a first-convergence region
(`distinct_entering_branch_count`), while SCC member count and repeated
entries from the same branch are provenance, not automatic extra
economic weight.**

`distinct_entering_branch_count(region)` (added by PATH-033,
`src/measurement.rs`) is the smallest possible candidate: it counts
`entries` deduplicated by `branch`, ignoring `members` entirely and
ignoring how many entries a single branch contributes. It is not wired
into any `aggregate_*` function or money allocation.

## What held up

Across `path_033_a` through `path_033_h`:

- **SCC-size inflation** (`a`): the same 2 branches entering a 2-member
  region vs. a 5-member region (same single entry point, longer
  downstream cycle) produce identical counts (2 == 2). H4's specific
  claim — member count is not automatic extra weight — held.
- **Repeated entries from one branch** (`b`): one branch touching the
  region at 2 members instead of 1 (3 entries instead of 2) does not
  change the count (2 == 2). H4's other specific claim held.
- **Genuine branch count changes are still detected** (`c`): 2 vs. 3
  actually-independent branches produce 2 vs. 3. The metric is not
  degenerate — it responds to the thing it claims to measure.
- **Redundant raw branches are correctly excluded when composed with
  PATH-030** (`d`): a raw branch subsumed per PATH-029 inflates the raw
  count (3) but not the effective-branch-composed count (2), on the same
  region membership. This confirms the metric must be composed with
  `effective_structural_branches` (not raw immediate branches) to avoid
  raw-count gaming, consistent with FIND-008's recommendation.
- **Duplicate edges, edge ordering, and renaming are inert** (`e`, `f`,
  `g`), matching every other primitive in this module.
- **Separate first regions are counted independently, no contamination**
  (`h`).

## The counterexample (`path_033_i`)

Two regions, same 2 distinct branches (`P`, `Q`) entering:

- Region A: `P->X`, `Q->X`, `X<->Y`. 2 members, 2 entries.
- Region B: `P->M1`; `Q->M1, Q->M3, Q->M5, Q->M7` into an 8-member cycle
  `M1..M8`. 8 members, 5 entries.

`distinct_entering_branch_count` returns **2 for both** — this is the
actual, current, unforced behavior, not a bug. The test locks it in and
is green.

This is not a topological defect (nothing here is nondeterministic,
order-dependent, or duplicate-inflated — the invariance tests above rule
that out). It is a real information loss, deliberately produced by H4's
own definition: a region where one branch reaches four separate points
inside an 8-node mutually-dependent blob is treated as economically
identical to a region where two branches each touch one shared node
once.

## Interpretation — H4 is not disproven, but is not shown sufficient

**H4 is not falsified as a claim about REDUNDANCY-LOSS weight
specifically.** Every test that isolated "how many independent branches
were lost" from other variables (`a` through `d`) supports treating
distinct branch count as a structurally well-behaved candidate measure of *that* dimension, and
`c`/`d` show it is not a degenerate/inert metric — it responds correctly
to real changes in branch count and correctly ignores raw-branch gaming.

**H4 is not shown to be economically SUFFICIENT on its own.**
`path_033_i` demonstrates that member count and entry multiplicity carry
information distinct branch count discards by construction — plausibly a
second economic dimension ("blast radius": how much capability surface
fails together once independence is lost) rather than noise. Whether
blast radius deserves independent economic weight alongside
redundancy-loss count, and if so how the two should combine, is a
modeling/economic judgment this codebase cannot resolve by topology
alone — it is not decided here, and `distinct_entering_branch_count`
should not be read as a finished answer to "how important is this
region," only to the narrower question "how many independent branches
converge here."

## What is NOT concluded

- This does not claim blast radius *must* carry weight — only that H4
  silently assumes it does not, and PATH-033 found no topological reason
  that assumption must be true.
- This does not propose a combined formula. Any such formula is a new,
  separately-justified economic design decision (as FIND-009 already
  flagged), not something PATH-033's structural tests can derive.

## Next architectural experiment

- If a future protocol decision commits to redundancy-loss-only weighting
  for convergence regions, `distinct_entering_branch_count` composed with
  `effective_structural_branches` (per `path_033_d`) is the tested,
  invariant candidate — cite `path_033_a` through `path_033_h` as its
  evidence base.
- Before wiring anything into `aggregate_*`, decide explicitly whether
  blast radius (`members.len()`, or something entry-multiplicity-aware)
  should be a second, separately-weighted term — `path_033_i` is the
  concrete evidence that this decision is live, not a formality.
- If that decision is made, add a dedicated PATH-0xx adversarial suite
  for the combined formula the same way PATH-033 tested this one:
  invariance to duplicates/order/renaming, and an explicit counterexample
  search before trusting it.
