# FIND-021 — Bounds + State Are Sufficient for Scalar Criticality Queries, Not for Explanation

Status: OBSERVED / DERIVED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-044

## Hypothesis under test (H15)

A multi-dimensional structural measurement can preserve epistemic truth by
carrying bounded uncertainty per primitive observable, and only deriving a
scalar Criticality result when the scoring/aggregation function is proven
compatible with those bounds.

## Production Criticality inventory (read before designing anything)

`CriticalityObservables { qualified_dependency_weight: f64 [0,1],
blocking_paths: u32, failure_blast_radius: u32, substitutability:
Substitutability }` → `measure_criticality(observables, weights) ->
CriticalityMeasurement { ..., score: f64 }`.

- **`score` is a weighted linear sum of four independently-normalized
  terms, each a function of exactly one observable (no cross terms):**
  `qualified_dependency_weight` used as-is; `saturating_count
  (blocking_paths, 5)`; `saturating_count(failure_blast_radius, 5)`;
  `substitutability_score(substitutability)` (fixed 4-level map: Easy
  0.00, Moderate 0.33, Difficult 0.67, None 1.00). This makes
  `measure_criticality` **separable and monotonic non-decreasing** in
  every one of its four inputs — the single fact this whole experiment
  turns on.
- `qualified_dependency_weight` has a real upstream derivation:
  `qualify_dependency(DependencyEvidence, weights) -> QualifiedDependency`
  (also separable, monotonic, continuous) feeding
  `aggregate_dependency_weight_with_model(&[QualifiedDependency], model)`,
  which has three real production models:
  - `Complementary`: `1 - Π(1 - score_i)`. Monotonic, continuous.
  - `Thresholded`: drops any dependency scoring `< 0.20` before the same
    accumulation. **A real, confirmed production discontinuity**
    (FIND-006; the regression suite's pre-existing
    `path_016_threshold_cliff_must_not_amplify_tiny_evidence_change`
    failure is exactly this). Still monotonic (never reverses direction),
    but not continuous.
  - `RankedDecay`: sorts scores descending, weights by `0.5^rank`, sums,
    clamps to 1.0. **Proved monotonic per-input in this experiment**
    (`path_044_ranked_decay_is_monotonic_probe`, with the closed-form
    argument in the test-file inventory comment): raising one
    dependency's score can only move it to an equal-or-better rank, and
    every adjacent-rank swap this triggers strictly increases the total
    (the swap only fires between a rising score and the smaller one it
    has just overtaken).
  - `deduplicate_structural_dependencies` /
    `aggregate_structural_dependencies_with_model` (FIND-007) sit one
    layer above, deduping `DependencyRecord`s by structural key before
    aggregation into `qualified_dependency_weight`.
- **No population-relative normalization exists anywhere in this
  pipeline.** Grepped for population/percentile/z-score/relative-rank in
  `measurement.rs`: zero hits outside `RankedDecay`'s own internal,
  single-capability rank, which never looks outside the slice passed to
  it. PATH-044-G is explicit N/A, backed by an executable check.
- `Substitutability`'s evidence-to-category derivation remains
  unspecified in production (FIND-011, unchanged); `blocking_paths` and
  `failure_blast_radius` have no derivation from a graph either
  (FIND-012) — all three are supplied as literal inputs at every call
  site, in production and in this experiment alike. No new dimensions
  were invented; every observable used here is an existing
  `CriticalityObservables` field.

## Candidates tested

**Candidate A — score each admissible world, then min/max.**
`path_044_candidate_a_score_each_world`: exact reuse of FIND-020's shape
(`Measured044 { lower, upper, state }`), computed by calling
`measure_criticality` once per enumerated admissible world.

**Candidate B — bound each observable independently, then score the
corners.** `path_044_candidate_b_corner_scores` /
`path_044_generate_corners`: computes per-dimension min/max across the
admissible worlds, then evaluates `measure_criticality` at every corner
of the resulting box (up to 2⁴ = 16).

**Candidate C — vector of independently bounded observables, no
scalar.** `MeasuredCriticalityObservables`: one `Bound<T>` per dimension,
each collapsing to `Supported`/exact independently, based only on
whether that specific dimension's own values agree across worlds.

**Candidate D — explicit admissible observable states.**
`AdmissibleCriticalityStates { states: Vec<CriticalityObservables>,
epistemic_state }`: retains the actual enumerated worlds; the scalar
range is *derived* from them (identical computation to Candidate A), but
the states themselves remain queryable.

All four are test-local (`tests/pathological_cases.rs`); no production
code was modified.

## OBSERVED

1. **Candidate A reproduces `measure_criticality` exactly for a single
   world** (`path_044_a`, `path_044_candidate_a_range_is_correct_...`).
2. **When only one dimension varies, Candidate A and Candidate B agree
   exactly** (`path_044_b`) — separability plus single-dimension
   variation means there is no correlation to lose.
3. **When two dimensions vary but only at the "all-low"/"all-high"
   diagonal (correlated at the extremes), Candidate A and Candidate B
   produce the identical scalar range** (`path_044_c`) — because
   `measure_criticality` is separable, the box's own extreme corners
   coincide exactly with the two admissible worlds. **But Candidate B's
   corner generator still fabricates non-admissible mixed-observable
   vectors as a byproduct of computing that range** — the range survives,
   the state space does not.
4. **When the admissible pair is anti-correlated (high-reach-but-
   contained vs. low-reach-but-maxed-out), Candidate B's range is
   strictly and substantially looser than Candidate A's true range**
   (`path_044_d`): true range `[0.38, 0.76]`, independently-bounded range
   `[0.17, 0.97]` — the box's own corners (`(reach low, everything easy)`
   and `(reach high, everything maxed)`) are themselves not admissible,
   yet dominate the true admissible extremes by monotonicity. Still
   *sound* (it soundly contains the truth), never falsely narrow — but
   more than double the true range's width in this example.
5. **Two different observable vectors can score identically using real
   production semantics** (`path_044_e`): trading dependency reach
   against substitutability ease (moderate reach + moderate
   substitutability vs. higher reach + easy substitutability) lands on
   the exact same `score`, via the actual weighted-sum arithmetic, not an
   invented equivalence.
6. **A scalar can be exactly Supported while its explanation stays
   Unresolved** (`path_044_i`): given the two same-scoring worlds from
   (5) as the only two admissible worlds, the scalar range collapses to
   exact/`Supported` (both worlds agree on the number), while a second,
   independent tracking of "do the worlds agree on the *vector*" stays
   `Unresolved` — they do not. This is the central distinction the
   mission asked for, and it required a field FIND-020's original
   `Measured { lower, upper, state }` does not have.
7. **The `Thresholded` model's cliff is real and large relative to the
   evidence delta that triggers it** (`path_044_f`): a 0.002 evidence
   delta (0.199 → 0.201) produces an output delta more than 50× larger
   under `Thresholded` than the identical evidence delta under
   `Complementary` on the same two points. The resulting Criticality
   `Measured044` range over the two discrete evidence points is a
   *correct* bound on those two exact points — the finding is that
   nothing licenses treating values *between* them as varying smoothly.
8. **No population normalization exists to attack** (`path_044_g`) — an
   unrelated capability's data existing elsewhere in the program
   provably cannot affect another capability's score, because no
   function in this pipeline accepts a population/peer-set argument at
   all.
9. **Conflict in one dimension survives all the way to the final
   scalar's state, without dilution** (`path_044_h`): `blocking_paths`
   Conflicted, the other three dimensions Supported (per-dimension,
   Candidate C) — the "worst-of" combination rule
   (`path_044_worst_state`) correctly propagates `Conflicted`, not
   `Unresolved`, to the final scalar `Measured044`, even though 3 of 4
   dimensions individually agree.
10. **Duplicate/replayed dependency evidence does not narrow the
    Criticality range**, reusing FIND-007's production dedup directly
    (`path_044_j`): ten replayed records under fresh
    dependency/path/domain ids per admissible world, same structural key,
    leave `qualified_dependency_weight` (and hence the derived
    Criticality range) byte-identical.
11. **Harmless representation depth (relay padding) does not change
    Criticality**, chaining FIND-017/FIND-020's already-proven graph
    invariant straight into the Criticality layer (`path_044_k`): a
    reach-derived `qualified_dependency_weight` proxy, computed via the
    unmodified production `canonicalize_pure_relays` +
    `reachable_capabilities`, is byte-identical bare vs. relay-padded,
    and so is the resulting Criticality score.
12. **Ranking stays epistemically honest across all three tested
    shapes** (`path_044_l`): disjoint ranges → a definite ranking;
    overlapping ranges → explicitly reported as unable to rank; and,
    reusing (14) below, numerically identical ranges arising from
    *different* admissible-state sets → also correctly reported as
    unable to rank (a tie), even though the two underlying situations are
    not the same — ranking-by-range only ever promises to compare
    numbers, and does so honestly.
13. **Composition over a shared, anti-correlated pair of admissible
    worlds never becomes narrower than the true joint range**
    (`path_044_m`), mirroring PATH-043-L's already-proven property,
    reconfirmed with real `measure_criticality` outputs on both sides of
    the sum.
14. **Candidate B's per-dimension bounding is quantifiably wasteful as a
    state-space representation**, even in cases where it does not distort
    the scalar range (`path_044_n`): 2 real admissible worlds across 4
    dimensions generate 16 corner combinations, of which only 2 (12.5%)
    are actually admissible — 87.5% of the generated state space is
    fabricated.
15. **The strongest counterexample: two admissible-state sets compress
    to byte-identical per-dimension bounds, byte-identical scalar
    min/max, and identical epistemic state, yet answer a legitimate
    downstream structural query differently** (`path_044_o`). Set 1 (2
    worlds, correlated at the diagonal) and Set 2 (the same 2 worlds plus
    both anti-correlated mixed worlds) produce identical `Candidate C`
    bounds and identical `Candidate A` scalar ranges — but "does an
    admissible world exist where reach is high AND blast radius is
    contained?" is `false` for Set 1 and `true` for Set 2. This is a
    direct, constructed proof that bounds+state (Candidates A, B, and C
    alike) are **not** a complete representation of the underlying
    admissible-state set — only Candidate D (retaining the actual
    enumerated states) answers it correctly.

## DERIVED

- **FIND-020 generalizes cleanly to the SCALAR query, for this specific
  (separable, monotonic) production scoring function, whenever the
  admissible world set is explicitly enumerable.** Candidate A/D's scalar
  range is exact where every admissible world agrees, correctly bounded
  where they disagree, correctly distinguishes Unresolved from Conflicted
  (via the same worst-of combination rule as FIND-020), is inert to
  duplicate/replay evidence and to representation depth, composes without
  manufacturing certainty, and supports epistemically honest ranking.
  This is a genuine, non-trivial generalization — it was not guaranteed
  in advance, because a naive "bound each dimension, then score" approach
  (Candidate B) is what most implementations would reach for first, and
  that one is NOT the same representation, even though for this
  particular separable-monotonic scoring function it happens to be sound
  (never wrong, only loose).
- **Separability is why Candidate B is sound rather than dangerously
  wrong.** Because `measure_criticality` has no cross-terms, evaluating
  the coordinate-wise min and coordinate-wise max ALWAYS produces a valid
  outer bound on the true admissible range, regardless of correlation —
  monotonicity guarantees the box extremes dominate every admissible
  point. Had any tested aggregation been non-monotonic in some dimension,
  Candidate B would have been unsound (capable of excluding a true
  extreme), not merely loose — no such function was found in this
  repository (all three `DependencyAggregationModel` variants are
  monotonic, `RankedDecay` non-obviously so; see inventory).
- **Correlation loss is a real, quantifiable cost of Candidate B, not
  merely a theoretical one, but it shows up in the STATE SPACE before it
  shows up in the SCALAR.** PATH-044-C shows the scalar range can survive
  independent bounding unscathed even while the corner generator
  fabricates impossible vectors; PATH-044-D shows the scalar range itself
  degrades once the admissible pair is not at the box's own extremes;
  PATH-044-N quantifies the fabrication directly (87.5% false state space
  in a 2-world/4-dimension example). The practical implication: Candidate
  B's looseness is not bounded by a small constant — it is bounded by how
  far the true admissible pair sits from the independent-bounds box's own
  corners, which is not knowable without already having the admissible
  set (i.e., without already having done what Candidate D does).
- **Scalar-value uncertainty and observable-vector uncertainty are two
  independent epistemic axes, and a single shared `state` field
  conflates them.** This is the mission's central distinction, and
  PATH-044-E/I make it concrete with real production arithmetic, not an
  invented scenario: two materially different operational postures
  (moderate reach + moderate substitutability vs. high reach + easy
  substitutability) can net the identical `score`. `Path044CriticalityResult
  { scalar: Measured044, explanation_state: Path044State,
  admissible_states: Vec<CriticalityObservables> }` is the minimal shape
  found here that keeps these separate: `scalar.state` answers "is the
  number settled," `explanation_state` answers "is the reason for that
  number settled," and they can legitimately disagree.
- **The minimum representation sufficient for the tested SCALAR queries
  (exactness, bounds, conflict-vs-unresolved, ranking, composition,
  duplicate/replay/representation-depth invariance) is `Measured044 {
  lower, upper, state }` computed by Candidate A/D over an explicitly
  enumerated admissible world set — identical in shape to FIND-020,
  merely re-derived over four observables instead of one.** The minimum
  representation sufficient for arbitrary downstream STRUCTURAL queries
  (PATH-044-O's co-occurrence question, and by the same argument any
  query about which observables can jointly hold) is strictly larger: the
  enumerated admissible set itself (`AdmissibleCriticalityStates`/
  Candidate D), because bounds of any kind — per-dimension (C) or scalar
  (A/B) — are a lossy projection of that set and PATH-044-O constructs two
  sets with identical projections and different truths.

## NOT ESTABLISHED

- **Whether the admissible world set is enumerable in a real deployment.**
  Every test here constructs `worlds: &[CriticalityObservables]`
  explicitly, exactly as PATH-043 did for reach. Nothing in this
  experiment addresses how a real system would derive that enumerable set
  from raw, possibly-unbounded identity/evidence ambiguity — FIND-018/019
  already showed that question is not fully answerable for identity
  facts specifically, and this experiment does not reopen it (per
  mission non-goals). If the admissible set is not enumerable (e.g.
  continuous, unbounded, or combinatorially intractable), Candidate
  A/D's scalar-range mechanism as tested here does not directly apply,
  and Candidate B's independent-bounds approach — despite its looseness —
  may be the only tractable fallback; this experiment does not test that
  regime.
- **Whether `RankedDecay`'s monotonicity proof generalizes beyond the
  single empirical probe tested.** `path_044_ranked_decay_is_monotonic_probe`
  checks one construction (a dependency jumping from last to first rank
  over four others); the informal closed-form argument in the code
  comment covers the general adjacent-swap case, but no property-based/
  exhaustive test was run.
- **Whether a genuinely non-monotonic or non-separable Criticality-
  adjacent aggregation exists elsewhere in this repository outside what
  was inventoried** (`measure_criticality`, `qualify_dependency`,
  `aggregate_dependency_weight_with_model`'s three models). None was
  found, but the inventory was scoped to the functions the mission named;
  `Contribution`/`distribution` scoring (which has its own
  `evidence_confidence` multiplicative factor — a genuine cross-term
  between confidence and every other factor) was explicitly out of scope
  and not inspected for monotonicity here.
- **Whether `Path044CriticalityResult`'s three-field shape (scalar +
  explanation_state + admissible_states) is minimal**, or whether
  `explanation_state` could be derived more cheaply than retaining the
  full vector set for every case (only PATH-044-O's specific
  co-occurrence-style query was tested; other downstream query shapes
  might need less, or more, than the full enumerated set).
- **Whether any of this is wired, or should be wired, into
  `CriticalityObservables`, `measure_criticality`, `Contribution`, or any
  `aggregate_*`/allocation function.** No such change was made or
  attempted, per the mission's explicit non-goals.
- **Whether Candidate B's looseness matters economically.** No monetary
  interpretation of the scalar range's width is proposed or implied
  anywhere in this experiment.

## Strongest surviving contract

For the tested class of queries — "what is the range of possible
Criticality scores across the enumerated admissible worlds, and is that
range settled, honestly uncertain, or actively disputed" — the surviving
contract is unchanged in *shape* from FIND-020:

`Measured044 { lower, upper, state }`, `state ∈ {Supported, Unresolved,
Conflicted}`, built by scoring every explicitly enumerated admissible
`CriticalityObservables` vector (never by independently bounding each
observable and scoring the corners) and taking the true min/max — proven
sufficient for PATH-044-A, B, C (scalar part), D (scalar part), F, H, J,
K, L, M given the separable-monotonic structure of `measure_criticality`
and its upstream `aggregate_dependency_weight_with_model` pipeline.

For queries about the EXPLANATION rather than the number, the minimal
sufficient contract is strictly larger:

`Path044CriticalityResult { scalar: Measured044, explanation_state:
Path044State, admissible_states: Vec<CriticalityObservables> }` —
required to state PATH-044-E/I's central distinction, and
`admissible_states` specifically required to answer PATH-044-O's
co-occurrence-style query, which no compression of it (bounds, per-
dimension or scalar) can answer in general.

## Strongest counterexample

`path_044_o_lossy_compression_counterexample`: two admissible-state sets
— a 2-world diagonal set and its 4-world all-corners superset — compress
to byte-identical per-dimension bounds (Candidate C), byte-identical
scalar min/max (Candidate A/B), and identical epistemic state, yet
disagree on whether an admissible world exists with high reach and
contained blast radius. This is a direct, executable proof that no
bounds-only representation (however many dimensions it separately
tracks) is a complete substitute for the enumerated admissible-state set,
for at least one class of legitimate downstream query.

## Outcome classification

**H15 is PARTIAL.**

CONFIRMED, narrowly, for the tested SCALAR Criticality queries over
`measure_criticality`'s actual (separable, monotonic) production
structure: bounded uncertainty per admissible world, combined via
FIND-020's exact `{lower, upper, state}` shape, correctly answers every
scalar-level requirement in PATH-044-A through N, INCLUDING the
correlation-sensitive cases (C, D) — because the scoring function's
separability makes Candidate A's true-world-set evaluation both correct
and (for the tested cases) equal to or tighter than Candidate B's naive
independent-bounds approach, never wider on the correct side, never
unsound.

REJECTED for the stronger claim that bounds (of any granularity —
scalar, per-dimension, or corner-evaluated) are a *complete*
representation of multi-dimensional Criticality under uncertainty:
PATH-044-O is a direct counterexample. The system must retain the
explicit admissible observable-state set (Candidate D) — or something
informationally equivalent to it — whenever a downstream query needs to
know more than "what values are possible," such as which combinations of
observables can jointly occur.

The mission's own "important distinction" (uncertainty of the scalar vs.
uncertainty of the underlying vector) is CONFIRMED as a real,
constructible phenomenon under actual production semantics (PATH-044-E),
not merely a theoretical concern — and confirms that generalizing
FIND-020's single-`state`-field shape naively to multi-dimensional
Criticality would silently lose information FIND-020 never had to
represent in the single-observable case, because a single observable has
no separate "explanation" to be unresolved about.
