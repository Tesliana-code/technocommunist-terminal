# FIND-023 — Joint Admissible States Are Necessary for Honest Normalization, and Production Has Real, Confirmed Gaps

Status: OBSERVED / DERIVED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-046

## Hypothesis under test (H17)

A normalized allocation result is epistemically justified only from the
joint admissible contribution state, not from independently normalized
marginal bounds. A contributor's allocation share may be reported as
exact Supported when that share is invariant across every jointly
admissible world relevant to the allocation, even if the underlying raw
contribution values are uncertain. When admissible worlds produce
different normalized allocation vectors, the system must preserve that
uncertainty and must not manufacture one precise allocation vector.

## Production allocation inventory (read before designing anything)

`allocate_contribution_pool(events, weights, pool) -> Result<Vec<
ContributionAllocation>, &'static str>` (`distribution.rs`):

- **The normalization formula is exactly `share_i = value_i /
  sum_j(value_j)`**, confirmed by direct inspection, matching the shape
  the mission asked to verify (not assumed).
- `value_i = ContributionEvent::calculate(weights)?.evidence_adjusted_value
  = base_work_value * weighted_factors * evidence_confidence`, where
  `weighted_factors` is a weighted sum of 4 factors (weights validated to
  sum to exactly 1.0, each in `[0,1]`) and `evidence_confidence` is
  itself validated to `[0,1]`.
- **Negative `base_work_value` is explicitly rejected** (`Err("base work
  value cannot be negative")`) — impossible by construction to reach
  normalization, confirmed in PATH-046-M.
- **Zero-total handling**: `total_contribution <= 0.0` → `Ok(Vec::new())`
  (an EMPTY vector — not one zero-amount entry per input event) if
  `contribution_pool == 0.0`, else `Err("cannot allocate a positive
  contribution pool with zero verified contribution")`. Normalization is
  UNDEFINED (rejected, not silently zeroed) whenever total contribution
  is zero and the pool is positive.
- **No deduplication of any kind exists for `ContributionEvent`.**
  `distribution.rs`/`contribution.rs` contain nothing analogous to
  `deduplicate_structural_dependencies` (FIND-007). **This is a real,
  confirmed gap**, demonstrated directly against production in
  PATH-046-J: a literal duplicate `ContributionEvent`, or the same
  content replayed under a fresh `contribution_id`, measurably shrinks
  every other contributor's share. Not fixed here (out of scope per this
  mission's explicit "do not modify production allocation behavior") —
  reported as an OBSERVED gap.
- **No rounding, cents, or currency-unit logic exists anywhere.** Pure
  `f64` throughout; every existing production test for this function
  already only checks conservation with an epsilon tolerance, never
  bit-exact.
- **Numeric domain gaps, confirmed against real production
  (PATH-046-M):** `base_work_value < 0.0` does not catch `NaN` (IEEE-754
  comparisons against NaN are always `false`), and there is no upper
  bound check at all, so both `NaN` and `f64::INFINITY` pass validation
  unrejected. `total_contribution <= 0.0` likewise does not catch `NaN`.
  A single `NaN` contributor silently poisons **every** contributor's
  `normalized_share` with `NaN`, returned inside `Ok(...)`, not as an
  error. An `f64::INFINITY` contributor makes its own share `NaN`
  (infinity/infinity) while every finite contributor's share silently
  becomes exactly `0.0` (finite/infinity), also inside `Ok(...)`.
- **Normalization is population-relative by construction**: the
  denominator is the sum over every event actually passed to the
  function. Adding or removing a contributor changes every other
  contributor's `normalized_share` without touching their own raw
  `contribution_value` at all (PATH-046-P). This directly **contrasts**
  with FIND-021/PATH-044-G, where Criticality had no population-relative
  concept anywhere in its pipeline — allocation's shared single
  denominator is structurally different from Criticality's per-dimension
  separable scoring.
- Output order is input order (confirmed in PATH-045-N, re-confirmed
  unchanged here) — no sort/comparison of any kind touches allocation.

## Candidates tested

**Candidate A — normalize midpoints/expected values.** Not built as a
separate function; killed by the same mechanism as PATH-045-J
(midpoint-of-one-representative-world is just one of several equally
admissible joint vectors — demonstrated directly by PATH-046-D/G, where
different joint pairings sharing the same marginals produce different
true allocation possibilities that no single representative world can
stand in for).

**Candidate B — normalize marginal endpoints independently.**
`path_046_candidate_b_marginal_share_bounds`: for contributor `i` with
marginal range `[lo_i, hi_i]`, `share_lo_i = lo_i / (lo_i +
sum_{j≠i} hi_j)`, `share_hi_i = hi_i / (hi_i + sum_{j≠i} lo_j)` — the
standard independent-interval-propagation-through-a-ratio enclosure.

**Candidate C — normalize every joint admissible world**, via the real
production formula (`path_046_joint_results`). Reference semantics for
this experiment.

**Candidate D — per-contributor share bounds + state**, computed from
the TRUE joint admissible set (`path_046_candidate_d_bounds`), not from
independent marginal extrema — the tight analogue of Candidate B's loose
enclosure.

**Candidate E — joint allocation state**, `Path046Eligibility` (`Final`,
`InformationalBounded`, `BlockedUnresolved`, `BlockedConflicted`,
`BlockedNonNormalizable`), derived from the required pathological cases
rather than assumed up front.

All are test-local (`tests/pathological_cases.rs`); PATH-046-J/M call
the real `allocate_contribution_pool` directly to observe production
behavior. No production code was modified.

## OBSERVED

1. **Exact control produces one exact vector and conserves the pool**
   (`path_046_a`): `(A,B,C)=(40,30,30)` → shares `(0.4,0.3,0.3)` exactly,
   `sum(allocated_amount) ≈ pool` within epsilon.
2. **Exact numerator, uncertain denominator: exact raw value does NOT
   imply exact share** (`path_046_b`): `A=40` in every world, but total
   swings `100 → 140` — A's share moves `0.4 → 0.286`.
3. **Uncertain raw values, invariant share: uncertain input does NOT
   imply uncertain queried result** (`path_046_c`): `(A,B) = (20,10)` and
   `(40,20)` — A's raw value doubles, but the 2:1 ratio (and hence share)
   is exactly preserved. Share is legitimately `Supported`/exact.
4. **Byte-identical marginal ranges, different joint allocation
   possibilities** (`path_046_d`): the exact PATH-045-C device (monotone
   vs. crossed pairing of the same two raw values on each side) applied
   to normalization — identical `marginal(A)`/`marginal(B)` across both
   sets, but A's true achievable share range differs between the
   correlated and crossed pairings.
5. **Candidate B is sound but loose, precisely quantified**
   (`path_046_e`): for the correlated (monotone-paired) set, the true
   range is strictly narrower than the independently-derived enclosure —
   the enclosure always contains the truth (sound) but is not tight
   whenever the actual admissible pairing avoids the cross-extrema
   combination Candidate B silently assumes.
6. **Per-contributor bounds do not form an allocation**
   (`path_046_f`): summing each contributor's own independently-sound
   upper bound (`A_hi + B_hi`) exceeds `1.0` (over 100%) — each interval
   is individually a valid projection of the true set; their collection
   is not itself a valid joint vector.
7. **Strongest counterexample: identical share bounds, different
   admissible joint vector sets, different legitimate downstream answer**
   (`path_046_g`): a 2-world set (A/B trade off, C fixed at 0.2) and a
   3-world set (the same two worlds plus a balanced `(0.4,0.4,0.2)`
   world) have byte-identical per-contributor min/max for A, B, and C —
   yet "can A and B simultaneously both receive at least 0.35?" is
   `false` for the first set and `true` for the second.
8. **Conflict propagates through the shared denominator — demonstrated,
   not assumed** (`path_046_h`): B's own raw value never changes, but
   its share still varies because A's Conflicted value swings the
   denominator; B's share correctly inherits `Conflicted`, not a
   downgraded `Unresolved`. A contrasting sub-case shows relevance CAN
   occasionally narrow — but only by numeric coincidence (an invariant
   ratio, reusing observation 3's mechanism with A now Conflicted), never
   by structural guarantee.
9. **Missing evidence is not treated as zero**
   (`path_046_i`): the naive "A absent" shortcut gives B a falsely
   precise `0.5`; the honest range must include both "A absent" and "A
   present with real evidence" worlds — `[0.3, 0.5]`.
10. **Duplicate/replayed evidence currently DOES inflate production
    contribution — a real, confirmed gap** (`path_046_j`), tested against
    the real function: both a literal duplicate `ContributionEvent` and
    the same content replayed under a fresh `contribution_id` measurably
    shrink every other contributor's share, because no dedup exists at
    all. This directly contradicts FIND-007's principle at this specific
    production boundary.
11. **Representation depth is inert once a value is derived through
    already-invariant machinery** (`path_046_k`): a reach-derived
    contribution value (via the unmodified production
    `canonicalize_pure_relays` + `reachable_capabilities`, exactly as in
    PATH-044/045-K) produces identical allocation shares bare vs.
    relay-padded.
12. **Zero-total worlds are handled explicitly, not invented**
    (`path_046_l`): all-zero + positive pool → the exact production
    error string; all-zero + zero pool → an EMPTY vec (confirmed, not a
    list of zero entries); a joint set spanning one normalizable and one
    non-normalizable world produces one `Ok` and one `Err` — no single
    unconditional verdict may be reported; this maps to a distinct
    `BlockedNonNormalizable` eligibility category, not `Unresolved`.
13. **Negative/NaN/infinity domain behavior, confirmed against real
    production** (`path_046_m`): negative is rejected; NaN and infinity
    are NOT rejected and silently corrupt the result inside `Ok(...)`
    (see inventory above for exact mechanics).
14. **Conservation is machine-approximate, never bit-exact by
    construction** (`path_046_n`): summed allocated amounts and summed
    shares both hold only within floating-point epsilon; no rounding or
    currency-unit logic exists to test, and none is invented here.
15. **Scale invariance holds against the real implementation**
    (`path_046_o`): `(A,B,C)` and `(3.7·A, 3.7·B, 3.7·C)` produce
    identical shares within epsilon.
16. **Allocation is confirmed population-relative, contrasting directly
    with Criticality** (`path_046_p`): adding contributor C changes A's
    and B's shares while their raw `contribution_value` fields are
    untouched; uncertainty about whether C even exists makes A's and
    B's shares a genuine range despite exact evidence for each of them
    individually.
17. **Exact individual share can coexist with a non-exact joint vector**
    (`path_046_q`): A's share is exactly `0.5` in both worlds
    `(50,30,20)` and `(100,40,60)` (A stays exactly half the total in
    each), while B's and C's individual shares genuinely vary against
    each other. A may legitimately be `Supported` while the overall
    joint allocation result is not.
18. **The settlement-eligibility boundary is derived and tested across
    all 7 listed cases** (`path_046_r`): cases 1 and 2 (one exact
    supported vector; multiple raw worlds with one invariant normalized
    vector) collapse to the SAME `Final` category — eligibility tracks
    OUTPUT exactness, not raw-input exactness. Case 3 (bounded,
    multiple vectors) and case 7 (exact-looking per-contributor bounds
    without the retained joint set) also collapse to the same category,
    `InformationalBounded` — per-contributor bounds, however tight,
    never earn `Final` without the actual joint vector set backing them.
    Cases 4/5/6 map to three distinct `Blocked*` categories.
19. **The compositional question is answered NO, mirroring PATH-045-M**
    (`path_046_compositional_question_*`): per-contributor allocation
    results derived from non-reconciled sources compose (by naive
    summation of independently-sound upper bounds) into a total
    exceeding 100% — the same mechanism as observation 6, now framed
    explicitly as the answer to whether separately-derived allocation
    results may be safely composed without a shared joint model. They
    may not.

## DERIVED

1. **What production allocation normalizes today**: `value_i /
   sum_j(value_j)`, applied to `evidence_adjusted_value` (base work value
   × weighted factors × evidence confidence), with no dedup, no
   rounding, and validated non-negative but not validated finite.
2. **Normalization IS population-relative** — the single most important
   structural contrast with FIND-021's Criticality inventory. Criticality
   is a separable per-dimension weighted sum with no shared denominator
   across dimensions; allocation is fundamentally a SHARED-denominator
   ratio across ALL contributors simultaneously. This is why relevance
   quarantine (PATH-043/044's disconnected-subgraph style structural
   irrelevance) does not exist for allocation: every contributor's share
   is, in principle, sensitive to every other contributor's raw value,
   through the one denominator they all share. Exactness under
   uncertainty (observations 3, 8's escape hatch, 17) is always a numeric
   COINCIDENCE (an invariant ratio) for allocation, never a structural
   GUARANTEE the way an unreachable subgraph was for reach/Criticality.
3. **Exact raw contribution can yield uncertain share** (observation 2)
   and **uncertain raw contributions can yield exact share**
   (observation 3) — both directions of H17's central claim are directly
   constructed with the real production formula, not merely asserted.
4. **Marginal contribution ranges are NOT sufficient** — proven three
   independent ways (observations 4, 6, 7): identical marginals compress
   away the pairing that determines the true achievable share set,
   independently-derived per-contributor bounds do not sum to a valid
   allocation, and two sets with identical bounds answer a legitimate
   joint query differently.
5. **Marginal SHARE ranges are equally insufficient**, for the same
   reason — Candidate D's tight per-contributor bounds (computed from
   the true joint set) are themselves shown, in observation 7, to be
   compatible with two different underlying joint vector sets. Tightness
   of the per-contributor projection does not rescue it from being an
   incomplete representation of the joint.
6. **Conservation, under actual numeric semantics, means**: `sum(share_i)
   ≈ 1` and `sum(allocated_amount_i) ≈ pool`, holding only to within
   floating-point epsilon (never asserted bit-exact anywhere in this
   experiment or in production's own existing tests) — because no
   rounding/currency-unit representation exists to make it exact.
7. **Zero-total worlds must be tracked as a DISTINCT epistemic category**
   from ordinary `Unresolved`/`Conflicted` uncertainty — not every
   admissible world is even guaranteed to produce a well-defined
   normalized vector, and a joint set spanning both kinds cannot receive
   any single verdict (exact, bounded, or otherwise) without first
   resolving whether the non-normalizable branch is genuinely admissible.
8. **Conflict propagates through the shared denominator by demonstrated
   mechanism, not by blanket assumption** — and the same mechanism
   explains why it CAN'T be structurally quarantined the way graph-based
   irrelevance could be in FIND-020/021: there is exactly one shared
   sum, so anything that affects any term potentially affects every
   other term's ratio. The only way relevance narrows is coincidental
   ratio-invariance, which must be checked per case, never assumed.
9. **The minimum representation sufficient for the tested allocation
   queries is `Path046Eligibility` derived from the actual enumerated
   joint admissible vector set** (Candidate C/D combined with explicit
   tracking of per-world normalizability and conflict) — per-contributor
   `Measured` bounds alone (Candidate B or D in isolation) are
   insufficient for any query that depends on which specific
   combinations of shares can co-occur (observation 7), matching
   FIND-021/022's prior finding one layer closer to money.
10. **What may safely cross into a future settlement layer**: only a
    `Final`-eligible result — a joint vector set that is fully enumerated,
    every world normalizes, and the normalized vector is identical
    across every world. Nothing about which of possibly-multiple raw
    worlds produced that vector needs to cross (observation 18's
    merge of cases 1 and 2) — this is the money-layer analogue of
    FIND-021's "explanation state doesn't need to cross for a
    value-only consumer."
11. **What must block settlement**: `BlockedUnresolved`,
    `BlockedConflicted`, and `BlockedNonNormalizable` all must block —
    none may be silently treated as `InformationalBounded`'s weaker
    "report a range" fallback, because in each of those three cases
    there isn't even a trustworthy range to report (missing evidence
    could resolve either direction; conflict must not be averaged; a
    non-normalizable branch has no share number at all for that world).
12. **"Per-contributor allocation bounds are not an allocation" is
    directly supported** — by observations 6 (bounds don't sum to a
    valid vector), 7/19 (identical bounds, different joint truths / a
    naive composition exceeding 100%), and 18 case 7 (bounds without the
    joint set can never earn `Final` eligibility). This mission's own
    "strongest desired counterexample" section is satisfied by
    observation 7.

## NOT ESTABLISHED

- **Whether the two confirmed production gaps (no dedup for
  `ContributionEvent`; NaN/infinity not rejected) should be fixed.**
  This mission explicitly forbids modifying production allocation
  behavior; both gaps are reported as OBSERVED facts about current
  behavior, not recommendations. Whether closing them is in scope for a
  future mission is not decided here.
- **Whether the joint admissible contribution-vector set is obtainable
  in a real deployment** for genuinely independent contributors, as
  opposed to the test-constructed sets used throughout this experiment.
  FIND-018/019 already established that independence/identity truth
  between two entities is not generally resolvable by evidence this
  repository has; this experiment assumes the joint set is given,
  exactly as PATH-043 through 045 did.
- **Whether `Path046Eligibility`'s five categories are complete** for
  allocation scenarios beyond the ones tested — e.g. a joint set where
  SOME but not all worlds are Conflicted vs. Unresolved simultaneously
  with SOME non-normalizable worlds was not constructed; only the
  pairwise combinations explicitly listed in PATH-046-R were tested.
- **Whether `stewardship`/`capital` allocation (which share the same
  `value/total` shape) inherit identical findings.** Not tested directly;
  `allocate_stewardship_pool` and `allocate_capital_pool` were inspected
  and confirmed structurally identical (same zero-total handling
  pattern, same lack of dedup, same lack of rounding) but no PATH-046
  test was run against them specifically.
- **Any monetary, escrow, guaranteed-minimum, reserve-holding, or
  partial-settlement policy.** Explicitly out of scope and not designed,
  per this mission's non-goals; the eligibility categories are
  informational classifications only.
- **Whether the NaN/infinity poisoning behavior is exploitable as an
  attack surface** (e.g. a malicious contributor submitting `NaN` to
  deny everyone else's allocation) versus merely a numeric-hygiene gap.
  Only the mechanism was demonstrated; no threat-model analysis was
  performed.

## Strongest surviving contract

`Path046Eligibility`, derived from:

1. If any relevant Conflicted evidence exists → `BlockedConflicted`
   (checked first — conflict must never be silently downgraded).
2. Else if any enumerated admissible world fails to normalize →
   `BlockedNonNormalizable`.
3. Else if the joint admissible set cannot be confidently enumerated at
   all (evidence missing) → `BlockedUnresolved`.
4. Else if only per-contributor bounds are available, without the
   retained joint vector set → `InformationalBounded` (never `Final`,
   regardless of how tight the bounds look).
5. Else, compute the real production `normalized_share` vector for
   every enumerated admissible world (Candidate C): if every resulting
   vector is identical → `Final`; otherwise → `InformationalBounded`.

Only `Final` permits a downstream consumer to treat allocation shares as
settled economic facts. Every other category permits, at most,
informational/bounded reporting, and three of the five categories must
block entirely.

## Strongest counterexample

`path_046_g_same_share_bounds_different_joint_vector_set`: a 2-world and
a 3-world joint admissible set share byte-identical per-contributor
min/max shares for A, B, and C, yet only the 3-world set admits a world
where A and B can simultaneously both receive at least 0.35. No amount
of per-contributor bound compression (marginal, or the tight
joint-derived Candidate D bounds) can recover this distinction — only
the retained joint vector set can. This directly establishes "per-
contributor allocation bounds are not an allocation," the mission's own
named target.

## Outcome classification

**H17 is CONFIRMED, narrowly — for the tested `value_i/sum(value)`
production formula, over an explicitly enumerable joint admissible
contribution-vector set — with two real, separately-important
production-gap findings that are neither part of H17's own claim nor
required to be fixed by this mission.**

All twelve success conditions are satisfied by the tested cases:
(1) normalization evaluated over actual joint states (C, D, G); (2) exact
shares survive uncertain raw inputs (C, H's escape hatch, Q); (3) exact
raw contribution does not imply exact share (B); (4) marginal bounds
cannot manufacture a joint allocation (D, F, G); (5) conservation checked
per admissible vector, epsilon-bounded (A, N); (6) conflict and missing
evidence remain visible, not averaged or zeroed (H, I); (7) duplicate/
replay evidence is inert IN THE TEST-LOCAL MACHINERY THIS EXPERIMENT
BUILT, but — separately and importantly — is confirmed NOT inert in the
real `allocate_contribution_pool` itself (J, the one place a "must not"
requirement was tested directly against production and found violated);
(8) representation depth is inert (K); (9) zero-total/non-normalizable
worlds handled explicitly, not invented (L); (10) scale invariance
confirmed against real production (O); (11) population-relative
denominator effects made explicit and contrasted with Criticality (P);
(12) a prospective settlement-eligibility boundary derived without
settling money (R).

This is Outcome A (CONFIRMED NARROWLY) for H17 as an epistemic-
representation claim. It sits alongside — but is logically independent
of — a separate, concrete production finding this mission's own
inspection discipline surfaced: **PATH-046-J shows FIND-007's
duplicate/replay-inert principle does NOT currently hold for
`allocate_contribution_pool`, unlike every other pipeline this
repository's PATH series has tested (structural dependencies, alias
evidence, Criticality's dependency-weight aggregation).** This is not a
counterexample to H17 (H17 is about what a joint-admissible-state
representation CAN preserve, and this experiment's own machinery does
preserve it); it is a reminder that the representation this mission
confirms sufficient is not automatically what production ALREADY does —
inventorying that gap, honestly and without fixing it, was explicitly
what this mission's "inspect first, report what production actually
does" discipline was for.
