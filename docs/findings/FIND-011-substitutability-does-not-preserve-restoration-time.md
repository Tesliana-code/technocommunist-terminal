# FIND-011 — Existing Substitutability Semantics Do Not Explicitly Preserve Restoration Time

Status: OBSERVED / HYPOTHESIZED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-034

This finding supersedes an earlier draft of the same investigation that
overstated its results (described that `Substitutability` behaved like a
binary `has_substitute: bool` predicate, and claimed necessity/graph
semantics the code does not support). This version corrects those
overclaims; see "Corrections from an earlier draft" at the end.

## Hypothesis under test (H5, corrected)

The existing `Substitutability` representation does not explicitly
encode or derive temporal restoration semantics. If counterfactual
criticality must distinguish otherwise-equivalent substitution scenarios
whose restoration latency differs, additional explicit temporal
evidence is required, because nothing currently derives or stores that
latency.

(`has_substitute = true/false` was only the motivating thought
experiment in the original mission brief -- a simplified illustration,
never an accurate description of the production representation.)

## OBSERVED — what the existing measurement actually is

Inspection of `src/measurement.rs` (PATH-027 through PATH-033) and
`tests/pathological_cases.rs`:

1. **`Substitutability` is NOT binary.** It is an existing four-level
   categorical enum (`Easy | Moderate | Difficult | None`) with a
   deterministic numeric normalization (`substitutability_score`:
   `Easy -> 0.00`, `Moderate -> 0.33`, `Difficult -> 0.67`,
   `None -> 1.00`). This normalization step is deterministic; only the
   evidence-to-category step (below) is not.
2. **The evidence-to-category derivation is unspecified.** No
   `Evidence -> Substitutability` constructor or documented rule exists
   anywhere in this module. This already falls under FIND-003 ("manual
   scores are not economic evidence") independent of this experiment.
3. **Because the derivation is unspecified, what Easy/Moderate/
   Difficult are meant to track is also unspecified** -- replacement
   time, replacement effort, functional completeness, cost, some
   combination, or something else entirely. This experiment cannot and
   does not resolve that ambiguity.
4. **No existing observable contains recovery/failover/replacement
   time in any explicit form.** No field, function, or PATH-027..033
   graph primitive (`StructuralConvergenceRegion`,
   `distinct_entering_branch_count`, etc.) reads or produces a temporal
   quantity.
5. **Graph structure (PATH-027..033) cannot supply this information
   either.** All of it is purely dependency-graph topology
   (nodes/edges/SCCs); real-world restoration latency is not a
   topological property of a graph, so no amount of additional
   traversal recovers it.
6. **PATH-027 through PATH-033 do not capture any part of this
   dimension.** They are exclusively about structural/topological
   redundancy (shared bottlenecks, first convergence, effective
   branches, distinct entering-branch count) -- orthogonal to
   restoration time.
7. **`Substitutability::None` has no established graph-structural
   meaning.** There is no graph-derived constructor for
   `Substitutability` at all, so `None` is treated here only as an
   existing categorical value with unspecified evidence derivation,
   exactly like the other three variants -- not as "no structural
   substitute path exists in the dependency graph," a meaning this
   codebase does not establish.

### What is explicitly NOT claimed from the above

- That Easy/Moderate/Difficult **cannot ever** reflect real temporal
  differences. A human assigning these categories manually could, in
  principle, assign different categories to a 30ms-failover case and a
  3-week-rebuild case. That would remain epistemically insufficient
  under FIND-003 (the derivation is still unspecified and
  unauditable), but it is a materially weaker claim than "the
  representation necessarily forces these two cases to the same
  value," which this experiment does not establish and does not rely
  on.
- That `Substitutability::None` has any specific graph-structural
  semantic.

## HYPOTHESIZED / candidate addition (PATH-034)

Added `TimeToEffectiveSubstitution` (`src/measurement.rs`):

```rust
pub enum TimeToEffectiveSubstitution {
    Bounded(std::time::Duration),
    Never,
}
```

- **`Bounded(Duration)`**: effective substitution is reached after an
  exact duration. `Duration::ZERO` is a valid, meaningful value
  (instantaneous / already-effective failover, PATH-034-E) -- distinct
  from "no substitution needed" or "no substitute exists," concepts
  this type does not model.
- **`Never`**: on the temporal axis only, no effective substitution is
  ever reached, on any horizon considered. `Never` says nothing about,
  and does not derive from, `Substitutability::None` or any other
  existing categorical value -- their evidence derivation remains
  unspecified, and no claim is made about what they mean or how they
  relate to this variant. `Never` exists so that "never effective" is
  representable instead of silently folded into `Bounded` with an
  arbitrarily large duration, or into `Substitutability::None`.

**This is offered as ONE candidate representation of explicit temporal
evidence, not the uniquely necessary one.** A richer future
representation -- e.g. a restoration trajectory over time -- could
encode the same distinction and derive a scalar like this one from it
instead of storing it directly. PATH-034 supports only the narrower
claim that *some* explicit temporal evidence is required if this
distinction must be preserved; it does not support that this specific
type is the correct or only way to supply it.

**Representation choice**: `std::time::Duration`, not a bespoke
integer-duration type. `Duration` stores an exact seconds+nanoseconds
count (no floating point), so equal durations expressed through
different units are provably equal (`Duration::from_secs(30) ==
Duration::from_millis(30_000)`, PATH-034-D). No `Instant`/`SystemTime`
is used, so no wall-clock/runtime-measurement or timezone semantics
leak into what is a deterministic protocol-level input value, not a
stopwatch reading.

**This type is NOT wired into** `CriticalityObservables`,
`measure_criticality`, any `aggregate_*` function, or `Contribution`.
It carries no weight, no normalization, and no combination rule with
`Substitutability` or anything else.

## PATH-034 cases implemented (`tests/pathological_cases.rs`)

- **A** — given EQUAL existing categorical input (test setup, not
  derived evidence -- the test does not claim the protocol would
  necessarily produce this equality), a differing `τ` still
  distinguishes the two cases. This demonstrates that categorical
  equality does not imply temporal equality, not that the protocol
  forces categorical equality in the first place.
- **B** — a non-`None` category with a fast bounded `τ` vs. the `None`
  category with `Never`: confirms adding the temporal axis does not
  erase the existing category distinctions. Makes no claim about what
  `None` means beyond being the existing distinct value.
- **C** — ordering (`τ1 < τ2 < τ3`) is preserved at the raw `Duration`
  observable level. No claim that a downstream scoring function must
  be monotonic or linear.
- **D** — unit invariance: `30s == 30_000ms == 30_000_000µs`, and the
  same equality holds when wrapped in `Bounded`.
- **E** — `Bounded(Duration::ZERO)` means instantaneous/already-
  effective failover; distinct from `Never` and from a merely-small-
  but-nonzero duration.
- **F** — a non-`None` category paired with `Never` is a representable,
  non-error combination. No claim is made about `Substitutability::None`
  or its relationship to `Never`; `Never` is defined purely on the
  temporal axis.
- **G** — graph invariance: identical topology, identical structural
  regions/`distinct_entering_branch_count`, differing only in attached
  `τ`. Confirms the temporal axis is orthogonal to -- and never
  consulted by -- the PATH-027..033 structural machinery.
- **H** — the converse: structurally different situations (2 vs. 3
  distinct entering branches) with an equal `τ` attached to both.
  Confirms temporal equality does not imply, and must not mask,
  structural inequality.
- **I** — counterexample: two situations with **identical `τ`** (both
  restore in 1 hour) where one restores full functionality and the
  other only partial functionality. This is the one counterexample
  dimension actually constructed and tested. It defeats `τ` as a
  *sufficient* substitutability measure on its own for at least this
  dimension.

## Adversarial counterexample found

- **`path_034_i`**: equal `τ`, different restoration completeness
  (full vs. partial) is implemented and tested. Other attack surfaces
  named in the original mission brief (manual intervention burden,
  capacity degradation, switching cost, technical-function-vs-
  business-outcome mismatch) are plausible by the same shape of
  argument but are **not implemented or tested** here, and are not
  claimed as established findings -- only noted as untested,
  similarly-shaped hypotheses for a future PATH.

## Verdict

**H5 SURVIVES IN NARROWED FORM.**

Established:

- `Substitutability` is a four-level manually supplied categorical
  input.
- Its numeric normalization (category -> score) is deterministic.
- Its evidence-to-category derivation is unspecified.
- No explicit temporal restoration observable existed before PATH-034.
- PATH-027..033 topology does not encode, and cannot be made to encode
  by further traversal, real-world restoration latency.
- A candidate exact-duration representation (`TimeToEffectiveSubstitution`)
  can preserve temporal distinctions that equal categorical input does
  not, and remains distinguishable from the existing `None` category
  without asserting what `None` means.
- Equal `τ` can still hide materially different restoration
  completeness (one tested counterexample dimension).

Not established:

- That Easy/Moderate/Difficult **cannot ever** reflect temporal
  differences under some unspecified human-assignment process.
- That `Substitutability::None` has graph-structural semantics.
- That `TimeToEffectiveSubstitution` is the uniquely correct temporal
  representation (a restoration-trajectory-derived scalar is an
  explicitly-noted alternative).
- That `τ` deserves economic weight.
- That `τ` should be normalized.
- That criticality is monotonic in `τ`.
- That any particular time horizon is correct.
- That restoration completeness (or any of the other named attack
  surfaces beyond the one tested) should become a protocol observable.
- Any combined criticality formula.

## What is NOT concluded

- No economic weight for `τ` is proposed.
- No formula combining `τ` with `Substitutability`, `distinct_entering_
  branch_count`, or anything else is proposed.
- `τ` is not claimed sufficient by itself (see `path_034_i`).
- No normalization, threshold, or time horizon is proposed.
- Nothing is wired into `measure_criticality`, any `aggregate_*`
  function, or `Contribution`. `Substitutability` and
  `TimeToEffectiveSubstitution` remain two separate, unweighted,
  unwired observables.

## Next architectural experiment

- If a future protocol decision commits to giving explicit temporal
  substitution latency economic weight, `TimeToEffectiveSubstitution`
  is one tested, invariant (units, ordering, zero-semantics,
  graph-orthogonality) candidate raw observable -- cite `path_034_a`
  through `path_034_h` as its evidence base -- but it is not shown to
  be the only possible representation, and a restoration-trajectory-
  derived alternative remains an open design choice.
- Before wiring anything in, `path_034_i` is the concrete open question
  that must be addressed first: whether restoration completeness (a
  `S_X(t) -> [0,1]`-style degree, per the mission's formal-model notes)
  or any of the other untested attack surfaces need to become a second,
  separately-justified temporal observable alongside `τ`, the same way
  FIND-010 left blast radius as an open second dimension alongside
  distinct branch count. Neither is decided here.
- If `Substitutability`'s evidence derivation is ever formalized (per
  FIND-003), that formalization should independently decide what
  Easy/Moderate/Difficult/None track -- this finding does not
  presuppose an answer, and any future derivation should be checked
  against PATH-034's tests for whether it still collapses cases that
  differ only in restoration latency.

## Corrections from an earlier draft

An earlier version of this document (previously titled "Substitutability
Requires Temporal Semantics") made claims not supported by inspection:

1. Described `Substitutability` as equivalent to a binary
   `has_substitute: bool` predicate. Corrected: it is an existing
   four-level categorical enum with a deterministic score mapping.
2. Claimed the existing categories "necessarily collapsed" 30ms-vs-3-
   week cases to the same value. Corrected: the evidence-to-category
   derivation is unspecified, so this could not be established either
   way; only the absence of *explicit* temporal encoding was
   established.
3. Claimed `Substitutability::None` means "no structural substitute
   path exists in the dependency graph." Corrected: no such
   graph-derived constructor exists; `None` is treated only as an
   existing categorical value with unspecified derivation.
4. Claimed `τ`/`TimeToEffectiveSubstitution` was "shown necessary."
   Corrected: only that *some* explicit temporal evidence is required
   if this distinction must be preserved -- this specific type is one
   candidate representation, not shown to be uniquely necessary.
5. Test and code comments referencing these claims were corrected in
   place (`src/measurement.rs`, `tests/pathological_cases.rs`); no
   test behavior or assertions changed except in `path_034_i`'s
   comment, which was narrowed to stop presenting untested attack
   surfaces as established findings.
