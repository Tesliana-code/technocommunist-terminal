# FIND-022 — All-Worlds Dominance Is a Sound Partial Order, But Requires Joint Admissible State

Status: OBSERVED / DERIVED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-045

## Hypothesis under test (H16)

A strict ranking `A > B` is justified only if every jointly admissible
world supports `A > B`. If some admissible worlds support `A > B` and
others support `B >= A`, the system must not manufacture a strict
ranking.

## Production ranking inventory (read before designing anything)

Grepped `src/*.rs` for rank/sort/compare/partial_cmp/total_cmp/priority/
normalized/share/allocation/criticality.

- **No production function compares two capabilities/contributors
  against each other economically, anywhere in this repository.**
  `allocate_contribution_pool` / `allocate_stewardship_pool` /
  `allocate_capital_pool` each compute `normalized_share = value /
  total` for every entry independently and `.map()` over the input
  slice — output order is INPUT order, not sorted by value. Confirmed
  executable in PATH-045-N: constructing events with a low-value
  contributor first and a high-value contributor second, the allocation
  output lists them in that same (non-value-sorted) order, even though
  the allocated *amounts* correctly differ.
- Every `sort_by` in `measurement.rs` (bottleneck lists, convergence
  regions, canonicalization candidates, dedup output, etc.) sorts by
  `CapabilityId`/`DependencyPathId`/`FailureDomainId` **string content**
  — deterministic output-enumeration stabilizers only. Several are
  already self-documented in-code as carrying "no structural or
  economic meaning" (e.g. `structural_propagation_surface`'s doc
  comment, inherited from PATH-037/FIND-014). None compares economic
  value.
- The one place raw `f64` scores are sorted is
  `DependencyAggregationModel::RankedDecay`
  (`scores.sort_by(|a,b| b.partial_cmp(a)...)`) — but this sorts ONE
  capability's OWN dependency list to aggregate it into ONE scalar; it
  never compares two different capabilities/contributors, so it is not
  a "ranking" in the sense this mission investigates.

**Conclusion: there is no existing production ranking/comparison
behavior to inventory further, attack, or (per the mission's non-goals)
modify.** This experiment builds a ranking relation from scratch,
test-local, reusing FIND-020/021's `Measured`/state machinery and real
`CriticalityObservables`/`measure_criticality` inputs where useful.

## Candidates tested

**Candidate A — compare midpoints.** `path_045_candidate_a_midpoint`.

**Candidate B — compare marginal bounds only** (no joint/correlation
information). `path_045_candidate_b_bounds_only`: `A.lower > B.upper` ⇒
`Greater`; `B.lower > A.upper` ⇒ `Less`; otherwise `Unresolved`.

**Candidate C — all-worlds dominance over the actual jointly admissible
`(score_a, score_b)` pairs.** `path_045_candidate_c_joint_dominance`:
`Equal` if every pair is equal; `Greater`/`Less` if every pair agrees on
strict direction; otherwise `Incomparable`.

**Candidate D — `RankingVerdict` result type + explanation state.**
`Path045Relation { Greater, Equal, Less, Incomparable, Unresolved,
Conflicted }`, built by `path_045_candidate_d_ranking_verdict`, which
applies Candidate C but lets an explicit `conflicted` flag override
everything else.

All four are test-local (`tests/pathological_cases.rs`); no production
code was modified.

## OBSERVED

1. **Candidate A is killed by a direct counterexample**
   (`path_045_j`): `A ∈ [0.3, 0.9]` (midpoint 0.6), `B ∈ [0.5, 0.55]`
   (midpoint 0.525) — midpoint claims `A > B`. But the actual jointly
   admissible world `(A=0.3, B=0.55)` has `B > A`. The true joint
   verdict is `Incomparable`, not `Greater`. Candidate A can assert a
   relation that does not hold in even a plurality of admissible worlds.
2. **Definite dominance survives cleanly when marginal ranges are
   disjoint** (`path_045_a`): `[0.8,0.9]` vs. `[0.3,0.5]` — both
   Candidate B and Candidate C (evaluated over all four corner
   combinations) agree on `Greater`.
3. **Overlapping marginal ranges correctly refuse a strict ranking from
   bounds alone** (`path_045_b`): `[0.4,0.8]` vs. `[0.6,0.9]` →
   `Unresolved` under Candidate B.
4. **Central counterexample: byte-identical marginal ranges, opposite
   joint verdicts** (`path_045_c`). Two joint sets built from the exact
   same two raw values on each side — `{(0.4,0.3),(0.6,0.5)}`
   (monotone-paired) vs. `{(0.4,0.5),(0.6,0.3)}` (crossed-paired) — have
   byte-identical `marginal(A) = [0.4,0.6]` and `marginal(B) = [0.3,0.5]`
   in both sets, and Candidate B returns `Unresolved` for both (it
   cannot see any difference). Yet Candidate C, using the actual
   pairing, returns `Greater` for the first set and `Incomparable` for
   the second. Marginal bounds provably cannot determine ranking here;
   only the joint/correlated pairing can.
5. **A perfect tie is reported as `Equal`, not `Unresolved`**
   (`path_045_d`): every jointly admissible world gives `A == B`
   exactly — a supported fact, not an absence of information.
6. **Exact scalar order can coexist with an unresolved explanation**
   (`path_045_e`), extending FIND-021 to ranking: `A` has two admissible
   observable vectors (moderate reach + moderate substitutability vs.
   higher reach + easy substitutability, same construction as
   PATH-044-E) that score identically; `B` is fixed and lower. Every
   joint pair — regardless of which of A's two explanations is real —
   has `A > B`. The ranking verdict is `Greater` (Supported/definite)
   while `A`'s own explanation state is `Unresolved`.
7. **Conflict remains visible even when the raw numbers look decisive**
   (`path_045_f`): `A`'s `blocking_paths` dimension is Conflicted
   between two grounded, disagreeing origins (mirrors PATH-044-H); both
   of `A`'s resulting admissible worlds score above `B` — a bounds-only
   check would confidently say `Greater`. The verdict is `Conflicted`,
   not `Greater`: conflict overrides even a seemingly-clean separation.
8. **Missing evidence is not treated as a low score**
   (`path_045_g`): `A`'s evidence for one dimension is simply missing,
   so both a low-scoring and a high-scoring world remain admissible for
   `A`. A naive shortcut that assumes "missing = the low world" would
   conclude `B > A` unconditionally. The correct joint verdict is
   `Incomparable` — genuinely undetermined, not a manufactured false
   defeat for `A`.
9. **Duplicate/replayed evidence does not upgrade ranking certainty**
   (`path_045_h`), reusing FIND-007's production dedup directly (as
   PATH-044-J did): ten replayed `DependencyRecord`s under fresh
   dependency/path/domain ids per admissible world leave the ranking
   verdict against `B` unchanged.
10. **Harmless representation depth does not change the ranking
    verdict** (`path_045_i`), chaining FIND-017/020's graph invariant
    (via the unmodified production `canonicalize_pure_relays` +
    `reachable_capabilities`) straight into a ranking comparison — bare
    vs. relay-padded graphs produce byte-identical derived
    `CriticalityObservables` and identical verdicts.
11. **Joint correlation justifies a strict ranking that marginal bounds
    alone refuse** (`path_045_k`): `{(0.55,0.5),(0.65,0.6)}` — marginal
    ranges `[0.55,0.65]` and `[0.5,0.6]` overlap (Candidate B:
    `Unresolved`), but every actual joint world has `A > B` (Candidate
    C: `Greater`). This is the same phenomenon as (4)/(6), isolated as
    its own minimal case per the mission's explicit emphasis.
12. **Candidate B's rule is sound whenever it fires — no counterexample
    exists to invent** (`path_045_l`). Proof: if `A.lower > B.upper`
    (both TRUE min/max over the full admissible set), then for ANY
    pairing `(a_i, b_j)`, `a_i ≥ A.lower > B.upper ≥ b_j`, so `a_i >
    b_j` regardless of correlation structure — the bounds are global
    facts independent of pairing. Tested against three scenarios with
    varying internal structure; every one where Candidate B fired,
    Candidate C agreed. Candidate B is conservative (PATH-045-K shows it
    under-detects), never wrong.
13. **All-worlds dominance is transitive only over comparisons that
    share one genuine joint admissible-world model — not when pairwise
    verdicts are composed from independently-derived pairwise sets**
    (`path_045_m`). Three separately-fabricated pairwise joint sets
    (`(A,B)` with `B ∈ {1,2}`, `(B,C)` with `B ∈ {0.5,0.6}` — a
    different, non-overlapping "B" — and `(C,A)` with `C ∈ {100,101}`)
    each individually and correctly yield `Greater`, composing into a
    cycle `A > B > C > A`. This is not a flaw in Candidate C itself —
    each pairwise verdict is correct for the joint set it was actually
    given — it is a demonstration that composing pairwise verdicts
    without a shared joint model silently assumes those separate models
    are mutually consistent, which nothing guarantees. By contrast, one
    shared 3-way admissible world set (`(A,B,C)` triples with `A>B>C` in
    every world) produces all three pairwise verdicts as `Greater` with
    no cycle possible, by ordinary pointwise real-number transitivity.
14. **No deterministic tie-breaking exists in production to test**
    (`path_045_n`), N/A, confirmed executably: allocation output order
    tracks input order, never value.
15. **The strongest lossy-compression counterexample**
    (`path_045_o`), reusing (4)'s construction: two joint sets with
    byte-identical `A` range, byte-identical `B` range, and identical
    epistemic state for both `A` and `B` produce different ranking
    verdicts (`Greater` vs. `Incomparable`). Per-side `Measured` values —
    however faithfully computed — are not sufficient for honest pairwise
    ranking.
16. **The allocation-boundary precondition table**
    (`path_045_p`): `Greater`, `Less`, and `Equal` are the only relations
    that may be consumed as an ordering fact by a hypothetical future
    allocation layer; `Unresolved`, `Incomparable`, and `Conflicted` must
    not be. This repository's actual allocation functions do not
    consume an ordering fact at all today (per the inventory) — the
    table is prospective guidance, not a change to existing behavior.

## DERIVED

- **When is `A > B` epistemically justified?** Only when every jointly
  admissible `(score_a, score_b)` pair has `score_a > score_b` —
  Candidate C's rule, exactly as H16 states. Marginal disjointness
  (`A.lower > B.upper`) is a SUFFICIENT special case of this (proof in
  observation 12), never a necessary one (observation 4/11 show joint
  dominance can hold even when marginals overlap).
- **When is `Equal` justified?** Only when every jointly admissible pair
  is exactly equal (observation 5) — not merely when marginal ranges
  coincide or overlap.
- **`Unresolved` vs. `Incomparable`, confirmed distinct and both
  necessary.** `Unresolved` means the representation available does not
  contain enough information to determine the relation — either only
  marginal (uncorrelated) bounds are available (Candidate B's native
  output), or evidence for one side is simply missing (observation 8).
  `Incomparable` means the joint/correlated admissible-state information
  IS available and it establishes, as a positive fact, that no single
  strict order holds across every jointly admissible world (observations
  1, 4, 8, 15). The distinction is about REPRESENTATIONAL COMPLETENESS
  at comparison time, not about whether the underlying truth could in
  principle be narrowed by future evidence — both states could
  hypothetically be narrowed by more information; what differs is
  whether the currently-available representation already contains a
  joint/correlated view. Collapsing this distinction would either (a)
  discard the fact that PATH-045-C/O's disagreement was actually
  *proven*, not merely unknown, or (b) claim knowledge (about
  correlation) that a marginal-only representation never had.
- **Are marginal scalar bounds sufficient?** No — proven insufficient
  three separate ways (observations 4, 11, 15) using the same core
  mechanism: identical marginal information compresses away the pairing
  that determines the true relation.
- **When is joint admissible-state information required?** Whenever
  marginal ranges overlap AND the caller needs anything stronger than
  "we don't know" — i.e., needs to distinguish a provable `Greater`
  (PATH-045-K) or a provable `Incomparable` (PATH-045-C/G) from mere
  `Unresolved`. When marginal ranges are disjoint, joint information adds
  nothing (observation 12) — Candidate B alone already gives the sound,
  correct answer.
- **Does all-worlds dominance form a partial order?** Yes, but only over
  comparisons drawn from ONE shared joint admissible-world model
  (observation 13, positive case). Ordinary pointwise real-number
  comparison is trivially reflexive-on-equality, antisymmetric, and
  transitive within a single shared model — there is nothing
  ranking-specific to prove there. The genuinely new finding is the
  NEGATIVE case: naively composing pairwise verdicts computed from
  separately-derived, non-jointly-reconciled pairwise evidence does NOT
  inherit transitivity, and can cycle, even though every individual
  pairwise verdict is individually correct for the (incomplete, mutually
  inconsistent) evidence it was given. This is a caller-discipline
  requirement, not a flaw in Candidate C: a ranking system must track
  whether its pairwise comparisons come from a shared joint model before
  chaining them.
- **Can exact score order coexist with unresolved explanation?** Yes,
  constructed directly with real production `measure_criticality`
  arithmetic (observation 6) — this is FIND-021's central distinction,
  now shown to survive being embedded inside a ranking comparison rather
  than only a single-value report.
- **What ranking information may safely cross into a future allocation
  layer?** Only `Greater`, `Less`, or `Equal` — i.e., only a relation
  proven to hold across every jointly admissible world (observation 16).
  Nothing about *why* (the explanation state) needs to cross that
  boundary for a purely value-proportional allocation mechanism like the
  one this repository actually has (PATH-045-N's inventory) — only the
  relation matters for such a mechanism, though a hypothetical future
  allocation mechanism that itself depended on *which* explanation is
  real (not just the aggregate value) would need `explanation_state` to
  cross the boundary too; this experiment's actual `allocate_*_pool`
  functions do not, so that extension remains hypothetical.
- **What must be rejected at that boundary?** `Unresolved`,
  `Incomparable`, and `Conflicted` — none of these may be silently
  treated as a tie, as "roughly equal," or as any other stand-in for a
  proven relation, per PATH-045-F/G/P.
- **Is the smallest necessary state set six values, as Candidate D
  offered?** All six were independently shown necessary by distinct
  constructed cases: `Greater`/`Less` (A, K — mirror images, both needed
  for an order-agnostic comparator signature), `Equal` (D), `Incomparable`
  (C, G, M, O), `Unresolved` (B, K's marginal-only view), `Conflicted`
  (F). No test collapsed any two of them. One related state — a WEAK
  "greater-or-equal-everywhere-but-not-always-strictly-greater-or-always-
  equal" relation — was never constructed or required by any mandated
  case, and is deliberately NOT added (see NOT ESTABLISHED); Candidate
  C's literal rule (H16's own wording) treats that shape as
  `Incomparable`, which this experiment does not contest for lack of a
  forcing case.

## NOT ESTABLISHED

- **Whether a 7th relation (weak/non-strict dominance — `A >= B`
  everywhere, with equality in at least one but not every world) is ever
  required.** No PATH-045-A through P case constructs this shape.
  Candidate C's literal rule (mirroring H16's own phrasing: "A > B only
  if forall worlds score(A) > score(B); A = B only if forall worlds
  score(A) = score(B); otherwise no strict ranking") reports this as
  `Incomparable`, which is defensible but not the only defensible
  choice — a system that needed to distinguish "genuinely mixed
  direction" from "never worse, sometimes tied" would need a richer
  relation set than the one derived here. Not built or tested.
- **Whether the joint admissible-world set is obtainable in a real
  deployment for two INDEPENDENT capabilities/contributors**, as opposed
  to two dimensions of the same Criticality computation (PATH-044) or
  two capabilities whose scores happen to derive from a shared,
  test-constructed evidence source (PATH-045-H/I). FIND-018/019 already
  established that identity/independence truth between two entities
  cannot generally be resolved by evidence this repository has; this
  experiment does not reopen that question (per its own non-goals) and
  simply assumes the joint set is given, exactly as PATH-043/044 assumed
  admissible world sets were given.
- **Whether `path_045_candidate_c_joint_dominance`'s pairwise-only
  transitivity gap (observation 13) has a cheap structural guard** — a
  way to detect, from the ranking layer alone, whether two pairwise
  verdicts share a joint model before composing them, without requiring
  the caller to already track that externally. Not designed or tested;
  this experiment only demonstrates the gap exists and shows the
  single-shared-model case as the fix, not an automatic detector.
- **Whether `Path045Relation` should be wired into, or should inform the
  design of, any actual ranking/comparison function in this
  repository.** None was made or attempted; per the mission's non-goals,
  no production ranking behavior was modified (there was none to
  modify).
- **Whether the allocation-boundary table (observation 16) generalizes
  to an allocation mechanism that is NOT purely proportional-share.**
  This repository's actual mechanism only needs the relation; a
  hypothetical rank-based or tournament-style allocation mechanism might
  need more (e.g. margin size, not just direction) — not modeled here,
  per the mission's explicit "no money formula" scope limit.

## Strongest surviving contract

`Path045Relation { Greater, Equal, Less, Incomparable, Unresolved,
Conflicted }`, computed by:

1. If a relevant Conflicted evidence state exists for either side,
   return `Conflicted` — overriding every other check (PATH-045-F).
2. Otherwise, if the actual jointly admissible `(score_a, score_b)`
   pairs are available, apply all-worlds dominance (Candidate C): return
   `Equal` iff every pair is exactly equal; `Greater`/`Less` iff every
   pair strictly agrees on direction; otherwise `Incomparable`.
3. Otherwise (only marginal, uncorrelated bounds are available), apply
   Candidate B: `Greater`/`Less` iff the marginal ranges are disjoint
   (provably sound per observation 12); otherwise `Unresolved`.

This relation is transitive and antisymmetric ONLY when composed within
one shared joint admissible-world model (observation 13) — that
precondition must be tracked by the caller, not assumed by the relation
itself. It forms a genuine partial order (never a forced total order)
over any set of comparisons that does share such a model.

## Strongest counterexample

`path_045_o_lossy_compression_counterexample` (built from `path_045_c`'s
construction): two joint admissible-state sets produce byte-identical
`Measured` values for both `A` and `B` — same range, same epistemic
state, on both sides, in both sets — yet the honest ranking verdict is
`Greater` in one and `Incomparable` in the other. No amount of
per-side compression (marginal bounds, scalar epistemic state, or both
together) can recover this distinction; only the joint/correlated
admissible-state pairing itself carries the information the ranking
question actually depends on.

## Outcome classification

**H16 is CONFIRMED, narrowly, as a partial-order relation over jointly
admissible state, with an explicit compositional caveat.**

All nine success conditions are satisfied for the tested cases:
(1) strict ranking returned only when valid across all jointly
admissible worlds (A, K, L); (2) supported equality distinguishable from
uncertainty (D vs. B/G); (3) conflict remains visible (F); (4) marginal
compression cannot fabricate certainty (C, K, O — Candidate B never
returns a wrong strict verdict, only an overly conservative
`Unresolved`, per the soundness proof in L); (5) joint-state correlation
is preserved and load-bearing exactly where ranking depends on it (C, K,
O); (6) ranking is invariant to duplicate/replay (H) and representation
depth (I); (7) transitivity is explicitly tested and shown conditional,
not unconditional (M); (8) deterministic sorting is confirmed entirely
separate from — and, in this repository, entirely ABSENT from — semantic
ranking (N); (9) an explicit safe handoff boundary to a future allocation
layer is defined and tested (P).

This is Outcome A (CONFIRMED NARROWLY) for the specific relation tested —
all-worlds dominance over an explicitly enumerable joint admissible-state
set, for real `measure_criticality`-derived scalar inputs — with one
qualification promoted from the mission's Outcome B: the relation's
USEFULNESS as a partial order across MORE than two items depends on
whether comparisons are drawn from one shared joint model, which is a
requirement on the CALLER (how comparisons are constructed and composed),
not a property this experiment found any way to enforce from inside the
relation itself. That caveat is the one respect in which "the rule works
pairwise, but richer [multi-party] semantics require additional
joint-state information" (the mission's own Outcome B language) is
partially true here — not because pairwise dominance is unsound, but
because composing independently-sourced pairwise verdicts is not,
by itself, guaranteed consistent.
