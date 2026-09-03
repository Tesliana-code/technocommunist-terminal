# FIND-013 — Structural Counterfactuals Do Not Determine Operational Degradation

Status: OBSERVED / HYPOTHESIZED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-036

> **PATH-036 EPISTEMIC CORRECTION (target-scope leak).** The first
> version of this finding described `StructuralCounterfactual` as
> reporting "how many effective structural branches `source` has,
> before/after" and called the result a "representation-invariant
> STRUCTURAL counterfactual" without flagging that its branch-count
> fields (`before_effective_branches`/`after_effective_branches`, as
> originally named) were **source-global**, not scoped to the queried
> `target` at all, while its reachability fields were target-specific.
> `path_036_m` confirmed this mixing produces a false target-associated
> signal: on `SOURCE->X->ANALYTICS, SOURCE->Y->PAYMENT`, querying
> `target = PAYMENT`, `removed = X`, the source-global branch count drops
> 2 → 1 purely because X's own (PAYMENT-irrelevant) branch disappeared,
> even though PAYMENT's redundancy is completely unaffected. This has
> been corrected in code: `structural_counterfactual`'s branch fields are
> now `before_effective_branches_to_target`/
> `after_effective_branches_to_target`, computed by a new
> `target_scoped_effective_branches(source, target, edges)` (existing
> `effective_structural_branches` filtered by `is_reachable`, never by
> label). The corrected version of this finding is below. Every "OBSERVED"
> item that depended on the pre-correction, unscoped fields has been
> re-verified against the corrected, target-scoped fields; nothing about
> the pre-correction overclaims should be relied upon or cited.

## Hypothesis under test (H7, corrected)

Original framing: "Pure reachability after removing capability X is
insufficient to represent system degradation whenever multiple paths,
redundancy, partial service, or threshold effects exist."

As actually tested and now precisely scoped:

**Binary target reachability can fail to preserve structural changes
visible elsewhere in the graph.** Specifically: `source -> target`
reachability can remain unchanged (`true` → `true`) after removing a
capability, while a target-scoped structural-redundancy count
(`target_scoped_effective_branches`) still detects a real loss of
independent `target`-reaching branches. This is deliberately narrower
than an earlier, overclaimed phrasing that pointed at the unscoped,
source-global `effective_structural_branches` drop as if it were
automatically evidence of a target-relevant change — `path_036_m` shows
that inference is false in general (a source-global branch drop can be
entirely irrelevant to the queried target).

## OBSERVED

1. **No generic removal/counterfactual helper existed before this
   mission.** Inspection of `src/measurement.rs` (PATH-027 through
   PATH-035) found no function that removes an arbitrary `CapabilityId`
   from a graph and recomputes downstream behavior. PATH-029's
   `effective_structural_branches` subsumption check performs an
   internal counterfactual removal, but it is scoped narrowly to
   deciding whether one of `source`'s own immediate branch roots is
   subsumed by another — it is not a general "remove X, ask about
   arbitrary target Y" utility. Every prior before/after test
   (PATH-030-d, PATH-033-d, etc.) hand-authored two separate edge lists
   rather than deriving one from the other.

2. **Four minimal deterministic primitives now exist**
   (`src/measurement.rs`, not wired into `CriticalityObservables`,
   `measure_criticality`, `Contribution`, or any `aggregate_*` function):
   - `remove_capability(edges, removed)` — the one removal semantics used
     throughout: delete every edge incident to `removed`, as source or
     target. No second, inconsistent removal notion was introduced.
   - `is_reachable(source, target, edges)` — plain cycle-safe BFS
     reachability.
   - `target_scoped_effective_branches(source, target, edges)` — added by
     the PATH-036 correction: `effective_structural_branches`
     (PATH-029/030), filtered to only the branches that can reach
     `target` at all, via `is_reachable`. Relevance to `target` comes
     from graph reachability, never from labels.
   - `structural_counterfactual(source, target, edges, removed)` — the
     before/after tuple `{before_reachable, after_reachable,
     before_effective_branches_to_target,
     after_effective_branches_to_target}`. **All four fields now describe
     `source`'s relationship to `target` specifically — none is a
     source-global quantity.**

3. **H7 (corrected) is confirmed for target-scoped reachability vs.
   target-scoped branch count (PATH-036-B/C/I).** On
   `SOURCE->X->PAYMENT, SOURCE->Y->PAYMENT`, querying `target = PAYMENT`
   and removing X, `after_reachable == before_reachable == true`, while
   `after_effective_branches_to_target (1) <
   before_effective_branches_to_target (2)`. This is established
   specifically for this topology, where the removed branch (X) itself
   lay on a `SOURCE -> PAYMENT` path — that is exactly what
   `target_scoped_effective_branches` requires before counting a branch
   at all, so the result is scoped by construction, not by coincidence
   of test design. A reachability-only observable would report no change
   at all here; the target-scoped branch-count primitive still records
   the loss of a `target`-reaching branch. Binary reachable/unreachable
   is therefore insufficient to preserve this structural change.

4. **Target-scoped structural redundancy is representation-invariant on
   the attacks tried (PATH-036-E/F/G/H).** `structural_counterfactual`'s
   target-scoped fields are unchanged under insertion of a stipulated
   pass-through relay on the surviving branch, duplicate edge
   observations, capability relabeling, and edge reordering — inherited
   directly from the existing `effective_structural_branches`/
   `build_structural_adjacency`/`is_reachable` dedup/traversal machinery,
   with no new invariance logic required. This claim is scoped to
   *target-scoped* redundancy, not "structural redundancy" generally —
   see item 6 for a case that same target-scoped count still misses.

5. **A false target-associated signal was confirmed, then closed by
   scoping (PATH-036-M).** On `SOURCE->X->ANALYTICS,
   SOURCE->Y->PAYMENT`, querying `target = PAYMENT` and removing X: the
   source-global `effective_structural_branches` count drops 2 → 1
   (confirmed), which — used directly, as the pre-correction type did —
   would misreport a PAYMENT-relevant redundancy loss. The corrected
   `target_scoped_effective_branches` (and `structural_counterfactual`
   built on it) correctly reports **no change** (1 → 1) for PAYMENT,
   because X's branch never had a path to PAYMENT. This is the target-
   scope leak this correction closes.

6. **The corrected, target-scoped `structural_counterfactual` still has a
   real blind spot (PATH-036-D/L), independent of the target-scope fix.**
   `shared_structural_bottlenecks` does NOT by itself distinguish
   `SOURCE->X->M->PAYMENT, SOURCE->Y->M->PAYMENT` (both branches share
   bottleneck M) from `SOURCE->X->PAYMENT, SOURCE->Y->PAYMENT` (truly
   independent) — PAYMENT itself is reachable from both branches in
   *either* shape and gets reported as "shared" in both, since that
   primitive does not exclude the queried target. The primitive that DOES
   distinguish them is `first_structural_convergences` (PATH-028): the
   shared-bottleneck shape has a first convergence strictly upstream of
   PAYMENT (at M), while the independent shape's only convergence is
   PAYMENT itself. In this specific pair of graphs both branches (X and
   Y) do reach PAYMENT, so `target_scoped_effective_branches` counts 2 for
   both shapes and the target-scope fix does not change the outcome:
   `structural_counterfactual(remove X)` still reports an **identical**
   result for both shapes, because removing X never removes M, and a
   target-scoped branch-count delta still cannot see that Y's surviving
   path funnels through the same shared bottleneck M. This is a
   genuine, still-open counterexample against the candidate, distinct
   from the target-scope leak: the two graphs are already
   distinguishable by an existing, previously-verified primitive that
   `structural_counterfactual` does not incorporate.

7. **Operational degradation degree is not representable at all
   (PATH-036-J).** The identical graph and identical
   `structural_counterfactual` result were computed under two stipulated
   scenarios — "Y has no spare capacity, so losing X halves served
   capacity" vs. "Y has full spare capacity, so losing X has no
   user-visible effect" — with the same result both times, because
   neither scenario's distinguishing fact (spare capacity) is
   represented anywhere in `DependencyEdge`/`CapabilityId`. Topology
   alone cannot determine operational degradation degree.

8. **No degraded-but-operational state exists anywhere in this module
   (PATH-036-K, recorded, not executable).** `StructuralCounterfactual`
   and every existing graph primitive express only boolean reachability
   and integer branch/convergence counts — there is no third state
   between "reachable" and "unreachable," no capacity fraction, no
   partial-service representation. This absence is recorded directly, as
   instructed, rather than modeled with a new state machine.

## HYPOTHESIZED

- That `first_structural_convergences` (or a function combining it with
  `structural_counterfactual`) could be composed into a richer candidate
  that also catches the PATH-036-D/L shared-bottleneck blind spot. This
  composition was not built or tested here — PATH-036 identifies the gap
  without closing it, per the mission's scope limits. (This gap is
  unrelated to, and survives, the target-scope correction.)
- That `structural_counterfactual`'s four-field shape generalizes cleanly
  to removal of any node, not just a source's own immediate branch —
  tested here only for `removed` at or near `source`'s immediate
  branches; deeper-graph removals were not separately attacked.
- That no further, undiscovered scope-leak-shaped bugs remain in
  `target_scoped_effective_branches` or `structural_counterfactual`
  beyond the one PATH-036-M attacked and closed. Only the specific
  "unrelated branch" attack shape was tried.

## NOT ESTABLISHED

- No operational degradation degree, capacity fraction, latency impact,
  or duration is computed, scored, or implied anywhere in this mission.
- No claim that "reachable means healthy," "two branches means
  redundancy" (independent of `target_scoped_effective_branches` actually
  confirming target-scoped independence), "unreachable means economically
  catastrophic," or "effective branch delta is criticality."
- No economic weight, severity score, or business-value ranking is
  established or wired in.
- `remove_capability`, `is_reachable`, `target_scoped_effective_branches`,
  and `structural_counterfactual` are **not** wired into
  `CriticalityObservables`, `measure_criticality`, `Contribution`, or any
  `aggregate_*`/allocation function.
- Whether `structural_counterfactual` composed with
  `first_structural_convergences` (or its cycle-aware/effective-branch
  variants) would close the PATH-036-D/L shared-bottleneck gap is not
  established — only that the current four-field shape, even after the
  target-scope correction, does not.

## Outcome classification

**Outcome 3 (partial).** Four minimal, deterministic, topology-only
primitives (`remove_capability`, `is_reachable`,
`target_scoped_effective_branches`, `structural_counterfactual`) give a
target-scoped, representation-invariant STRUCTURAL counterfactual —
target reachability and target-scoped effective-branch count, before and
after removal — built almost entirely from reused PATH-027..035 machinery,
plus one small filtering composition added by the PATH-036 correction.
This structural counterfactual is demonstrably more informative than
target reachability alone (PATH-036-C/I), and — after the correction — no
longer conflates a source-global branch count with a target-specific
question (PATH-036-M). It remains demonstrably NOT a complete structural
summary even after that correction (PATH-036-D/L show it still collapses
a real shared-bottleneck distinction that an existing primitive,
`first_structural_convergences`, can already see), and it says nothing
about operational degradation degree (PATH-036-J/K).
