# FIND-020 — Bounded + Epistemic-State Measurement Preserves Structure Without False Precision

Status: OBSERVED / DERIVED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-043

## Hypothesis under test (H14)

A structural measurement can preserve useful known information while
carrying explicit epistemic uncertainty separately, such that unresolved
identity or dependency facts cannot silently produce a fully precise
scalar result.

## Background

FIND-018 (PATH-041) and FIND-019 (PATH-042) jointly established an
epistemic boundary this mission takes as given, not reopened: identity
truth between two admissible graph interpretations cannot be established
by any topology-only or self-issued evidence primitive in this
repository, and even an explicit exogenous trust axiom cannot be
distinguished from a forged one once supplied. Some facts must therefore
legitimately remain SUPPORTED, UNRESOLVED, or CONFLICTED. PATH-043 asks a
narrower, downstream question: given that boundary, what may the
*measurement layer* (dependency reach, criticality-like observables)
honestly return, without fabricating precision, discarding known
structure, collapsing uncertainty to zero, or rewarding uncertainty as if
it were evidence?

## Candidates tested

**Candidate A — force one scalar.** Compute from whichever graph happens
to be materialized; ignore that a second, equally admissible
interpretation exists.

**Candidate B — blanket Unknown.** If any relevant unresolved fact
exists anywhere, return `None`.

**Candidate C — bounds only.** `{lower, upper}` computed as the min/max
of the queried value across every admissible world, with no accompanying
epistemic tag.

**Candidate D — structural result + epistemic state.** Test-local
`Measured { lower, upper, state }`, `state ∈ {Supported, Unresolved,
Conflicted}`, built by `path_043_from_world_values` /
`path_043_measure_reach` (`tests/pathological_cases.rs`). `lower`/`upper`
are computed directly from the queried value observed in each admissible
world; `state` collapses to `Supported` whenever every world agrees,
*regardless of why multiple worlds exist* — this is what makes relevance
sensitivity (PATH-043-C) fall out of the mechanism itself rather than
requiring a separate rule. Separately, `path_043_relation_state` decides
*whether* more than one world is admissible at all for a given identity
pair, from raw alias evidence (`Path043AliasEvent`), using the same
content-addressed dedup as FIND-007/PATH-042 Candidate C (a free-form
`event_id` is never trusted as identity) plus PATH-042-B's self-observed
filter. No probability, confidence coefficient, or manual score appears
anywhere in this module.

All four candidates are test-local (`tests/pathological_cases.rs`); no
production code (`measurement.rs`) was modified. `path_043_measure_reach`
calls the unmodified production `canonicalize_pure_relays` and
`reachable_capabilities` to compute each world's value.

## OBSERVED

1. **Candidate A produces an exact-looking scalar that is silently
   conditional on an unverified assumption**
   (`path_043_candidate_a_forced_scalar_depends_on_an_unverified_representation_assumption`).
   `dependency_reach = 4` on the materialized "independent" world versus
   `3` on the equally admissible "alias" world — Candidate A has no
   representation for the fact that a second answer exists at all.

2. **Candidate B discards a provable minimum**
   (`path_043_candidate_b_blanket_unknown_discards_provable_structure`).
   Once any relevant ambiguity exists, Candidate B returns nothing, even
   though every admissible world in the same fixture establishes a
   nonzero minimum reach (`measured.lower > 0`) that is true regardless
   of how the ambiguity resolves.

3. **Candidate C conflates two epistemically different situations that
   must not be handled the same way**
   (`path_043_candidate_c_bounds_alone_conflate_unresolved_with_conflicted`).
   An honestly unresolved pair (missing evidence) and an actively
   disputed pair (two grounded, non-self origins disagreeing) produce the
   byte-identical `{lower, upper}` pair; only Candidate D's `state` field
   distinguishes "safe to treat this range as honest ambiguity" from "two
   authorities are actively disputing this — never silently trust it."

4. **Candidate D preserves exactness when nothing is ambiguous**
   (`path_043_a`). A single admissible world (no relevant unresolved
   identity) yields `lower == upper`, `state == Supported` — PATH-043-A's
   control.

5. **Candidate D refuses one unconditional exact scalar under genuine
   ambiguity** (`path_043_b`). The FIND-018/019 minimal shape
   (`source -> x -> m`, `source -> y -> m`) with no relation evidence at
   all yields `Measured { lower: 2, upper: 3, state: Unresolved }`, not a
   single number.

6. **Uncertainty propagation is relevance-sensitive by construction, not
   by a bolted-on rule** (`path_043_c`). The same x/y ambiguity present
   elsewhere in the graph does not affect a query whose reachable set
   never touches x, y, or m — `path_043_measure_reach` collapses to
   `Supported`/exact because the *value itself* does not vary across
   worlds, independent of whether ambiguity exists somewhere else in the
   same graph.

7. **Lower and upper bounds survive independently**
   (`path_043_d`, `path_043_e`). Worlds disagree (4 vs. 5), but the
   provable minimum (4) and provable maximum (5) are both preserved as
   named fields, not discarded in favor of picking one side.

8. **Conflict is surfaced explicitly and never averaged into a scalar**
   (`path_043_f`). Two distinct, non-self-derived origins asserting
   opposite alias facts produce `state == Conflicted`; the underlying
   `lower != upper` is retained (2 and 3), with no mechanism anywhere in
   `Measured` to combine them into one reconciled number.

9. **Missing evidence is not treated as false**
   (`path_043_g`). Zero relation claims yields `Unresolved`, not a
   default resolution to "distinct." A naive candidate that treats
   silence as proof of independence commits to the wrong world's exact
   value (3) and silently discards the other admissible world's value
   (2) that Candidate D's `lower` field correctly still reflects.

10. **Duplicate evidence does not narrow the interval or improve the
    epistemic state** (`path_043_h`). Ten literal duplicates of the same
    one-sided claim content-dedup to one event (direct FIND-007 reuse)
    and produce the identical `Unresolved` state as a single copy.

11. **Replay under a fresh event id does not narrow or upgrade
    certainty** (`path_043_i`). Five replays of identical content under
    five different `event_id`s dedup to one event; state is unchanged
    from the single-copy baseline.

12. **Harmless representation depth leaves measurement and state
    invariant** (`path_043_j`). Inserting a `RepresentationOnly` relay
    into either branch changes nothing, because `path_043_measure_reach`
    canonicalizes every world via the unmodified production
    `canonicalize_pure_relays` before measuring — this experiment
    consumes FIND-017's already-established invariant rather than
    re-deriving it.

13. **Claim volume cannot launder Unresolved into Supported**
    (`path_043_k`). Thirty claims (ten self-derived, twenty duplicate
    content from one grounded actor) yield the identical `Unresolved`
    state as a single relevant claim — matching FIND-018/019's repeated
    observation that claim count is free for a single actor to
    manufacture.

14. **Naive interval-arithmetic composition never becomes narrower than
    the truth, and preserves exactness when justified** (`path_043_l`).
    Composing two anti-correlated `Measured` values via `{lower1+lower2,
    upper1+upper2}` produces a range that is provably at least as wide as
    the true joint range computed by summing within each shared
    admissible world (`composed.lower <= true_joint.lower`,
    `composed.upper >= true_joint.upper`) — over-approximation, never
    false narrowing. Composing two exact/`Supported` inputs yields an
    exact/`Supported` composite (`5+7=12`, exactly).

## DERIVED

- **The smallest representation that survived every attack in this
  experiment is `{lower, upper, state}`**, where `lower`/`upper` are
  computed per query from the actual value observed in each admissible
  world (not stored as a static property of the graph), and `state` is
  derived from two independent facts: whether `lower == upper` for THIS
  query, and — only when they differ — whether the surrounding ambiguity
  is a genuine two-sided grounded disagreement (`Conflicted`) or anything
  else, including total silence (`Unresolved`).
- **Relevance sensitivity is not a separate filter bolted onto the
  representation — it is a structural consequence of computing bounds
  per query, over the actual admissible worlds, rather than propagating a
  single graph-wide "has ambiguity" flag.** This is the answer to
  PATH-043-C and to the mission's relevance-sensitivity question: any
  design that first computes "is anything unresolved in this graph" and
  only then decides whether to trust a specific number will fail
  PATH-043-C; a design that computes the number in every admissible world
  first and asks whether those numbers agree will not.
- **Bounds and epistemic state answer different questions and both are
  required.** Bounds alone (Candidate C) tell a caller what values are
  possible; they cannot tell a caller whether the disagreement is safe
  ambient uncertainty or an active dispute that should block a decision
  entirely. Candidate D's `state` field is exactly the piece of
  information Candidate C is missing (OBSERVED 3), and it is exactly the
  piece FIND-018/019 already showed cannot be upgraded into a *resolved*
  fact by any evidence this repository can construct — so `state` is
  reported, never resolved further.
- **What the measurement layer can state exactly, when the underlying
  representation is epistemically ambiguous:** the true minimum, the true
  true maximum, and an honest, non-numeric account of why those two
  might differ (or an explicit signal that they don't, for this specific
  query). It cannot state, and this experiment does not attempt to
  produce, a single number standing in for "the most likely value" —
  that would require the probability model this mission explicitly
  forbids inventing.

## NOT ESTABLISHED

- **Whether `{lower, upper, state}` is sufficient for richer criticality
  semantics** (`CriticalityObservables`/`measure_criticality`, which
  combines four weighted, saturating dimensions into one `score: f64`).
  PATH-043 tested only reach-like count observables
  (`reachable_capabilities`). Whether a `Measured<CriticalityMeasurement>`
  built the same way (per-dimension bounds, then weighted-combined) would
  need interval arithmetic through `saturating_count` and the weighted
  sum, and whether THAT composition still avoids false precision, was not
  tested. This is an explicit instance of the mission's Outcome B
  ("bounds work for reach/count-like observables but fail for richer
  criticality semantics or composition") that was not ruled in or out.
- **Whether `path_043_relation_state`'s two-tier state (missing evidence
  = Unresolved; grounded two-sided disagreement = Conflicted; anything
  else = still Unresolved) generalizes beyond a single pairwise identity
  question.** Only one relation (x vs. y) was tested at a time; multiple
  simultaneously-ambiguous relations feeding the same query were not
  constructed, though `path_043_l`'s two-input composition is one step in
  that direction.
- **Whether naive interval-sum composition's over-approximation is tight
  enough to be useful at scale.** PATH-043-L shows it never falsely
  narrows, but repeated composition of many correlated uncertain inputs
  could widen bounds enough to make them uninformative; no bound on that
  widening was measured, and no correlation-aware composition (which
  would require knowing the shared world set, not just each input's
  published `{lower, upper}`) was built beyond the single illustrative
  case in `path_043_l` where the true joint value was computable because
  the shared per-world values were still in scope.
- **Whether this representation is wired, or should ever be wired, into
  `Contribution`, `CriticalityObservables`, `measure_criticality`, any
  `aggregate_*` function, or allocation.** No such change was made or
  attempted; this remains test-local per the mission's non-goals.
- **Whether an economically meaningful decision can safely consume a
  `Conflicted` result at all** (e.g., should downstream code refuse to
  proceed, refuse only monetary consequences, or something else). This
  experiment only establishes that `Conflicted` must be surfaced, not
  what a caller should do with it.

## Strongest surviving contract

`Measured { lower, upper, state }`, built per query from the actual value
observed in every currently-admissible world (never from a static
graph-wide "has ambiguity" flag), where:

- `state = Supported` and `lower == upper` whenever every admissible
  world agrees for *this specific query*, regardless of whether ambiguity
  exists elsewhere in the graph (PATH-043-A/C);
- `state = Unresolved` when worlds disagree and no grounded two-sided
  disagreement exists — including total silence (PATH-043-B/D/E/G);
- `state = Conflicted` when worlds disagree AND at least two distinct,
  non-self-derived, content-deduplicated evidence origins actively assert
  incompatible facts about the same relation (PATH-043-F);
- both `lower` and `upper` are always the true minimum/maximum across
  admissible worlds, never dropped in favor of the other (PATH-043-D/E);
- neither `lower`, `upper`, nor `state` can be moved by duplicate
  evidence, replayed evidence under a new id, claim-count inflation, or
  harmless representation-depth padding (PATH-043-H/I/J/K), because the
  world set and the relation-evidence dedup both reuse
  FIND-007/FIND-017/FIND-018/FIND-019's already-established invariants
  rather than re-deriving weaker ones;
- composition via interval arithmetic never produces a range narrower
  than the true joint range, and never manufactures uncertainty where
  none exists (PATH-043-L).

## Strongest counterexample

`path_043_candidate_c_bounds_alone_conflate_unresolved_with_conflicted`:
an honestly-unresolved pair and an actively-disputed pair produce
identical `{lower, upper}` output. Bounds alone are structurally
insufficient — not merely incomplete — for any caller that needs to
decide whether a range is safe to treat as ordinary ambient uncertainty
or must block on an active dispute. This is the concrete reason Candidate
D (bounds + state) is necessary and Candidate C (bounds only) is not,
mirroring the mission's own warning against conflating "value ∈ [3, 7]"
with "value = 5 with 60% confidence": bounds-only representations invite
exactly that conflation by discarding the one field that would prevent a
caller from silently treating a dispute as safe.

## Outcome classification

**H14 CONFIRMED, narrowly, for count-like structural observables
(dependency reach).** `{lower, upper, state}`, built per query from
admissible-world enumeration rather than a global uncertainty flag,
satisfies all nine items in the mission's success condition list for
every required case PATH-043-A through L: it preserves exactness when
supported (A, L-exact); it refuses false precision under disagreement
(B, Candidate A attack); it preserves both bounds independently (D, E);
it keeps Unresolved distinct from Conflicted (F vs. G/B, Candidate C
attack); it is invariant to duplicate/replay inflation (H, I) and to
harmless representation depth (J); it introduces no arbitrary confidence
discount anywhere (no candidate in this module multiplies a value by a
coefficient); it composes without silently erasing uncertainty (L); and
it downgrades only measurements actually affected by the uncertainty (C),
with that last property falling directly out of the per-query world-value
mechanism rather than a separately-coded relevance rule.

This is Outcome A (NARROW CONFIRMATION) for the specific observable
class tested — reach/count-like measurements over a graph with a single
binary identity ambiguity. It is explicitly **not** shown to extend to
`CriticalityObservables`'s weighted multi-dimensional `score: f64`, to
more than one simultaneously-relevant ambiguity, or to any production
wiring — those remain open per NOT ESTABLISHED, consistent with the
mission's Outcome B still being a live possibility one layer up, and
consistent with H14 as stated ("a structural measurement can preserve
useful known information while carrying explicit epistemic uncertainty
separately") being a claim about representational shape, not a claim that
every measurement in this repository already has this shape.
