# FIND-012 — Failure Blast Radius Lacks a Representation-Invariant Derivation

Status: OBSERVED / HYPOTHESIZED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-035

## Hypothesis under test (H6)

Raw affected-node count is not a representation-invariant measure of
failure blast radius if one real-world affected capability can be
represented as either one graph node or several internal graph nodes
(representation laundering).

## OBSERVED

1. **`CriticalityObservables.failure_blast_radius: u32` has no
   derivation.** No `Evidence -> FailureBlastRadius` constructor exists
   anywhere in `src/measurement.rs`. `measure_criticality` only consumes
   the value (`saturating_count(value, 5)`, then a linear weight); it
   never computes it from `DependencyEdge`, `CapabilityId`, or any other
   graph primitive. Every call site (PATH-012, and the `measurement.rs`
   unit tests) supplies it as a literal integer. This is the same shape
   of gap as `Substitutability` (FIND-011 / PATH-034): the problem is
   **absence of a deterministic, representation-invariant measurement
   function**, not evidence that an existing derivation performs naive
   node counting. `failure_blast_radius` does not compute a count of
   anything today.

2. **H6 is real against a naive strawman, tested and confirmed
   (PATH-035-A, PATH-035-B).** A test-only `naive_downstream_reachable_count`
   function (plain BFS over `DependencyEdge`, deduplicated by a visited
   set) was constructed solely to give H6 something concrete to attack,
   since no production naive-counting function exists to attack
   directly. Against it:
   - Node splitting (`X->PAYMENT` vs. `X->PAYMENT_FRONTEND->PAYMENT_BACKEND`,
     stipulated as the same capability): naive count inflates 1 -> 2.
   - Relay insertion (`X->A->B` vs. `X->A->RELAY->B`, RELAY stipulated as
     no independent capability): naive count inflates 2 -> 3.

3. **An existing PATH-029/030 primitive is invariant to both attacks,
   with zero new production code.** `effective_structural_branches(source,
   edges).len()` — built for a different purpose (subsumed-branch
   detection ahead of convergence analysis) — gave equal counts on both
   node-splitting and relay-insertion pairs (PATH-035-A, PATH-035-B), and
   remained equal under duplicate-edge representation (PATH-035-C,
   already neutralized by existing edge dedup, consistent with FIND-007),
   edge reordering (PATH-035-G), and capability relabeling (PATH-035-F).
   It also does not degenerate to a constant: two genuinely independent
   downstream capabilities (`X->PAYMENT`, `X->SHIPPING`) produced a
   strictly larger count than one (PATH-035-D: 2 > 1).

4. **`FailureDomainId` (PATH-024/025/026) does not bear on this
   question.** That machinery dedupes `DependencyRecord`/
   `QualifiedDependency` for the economic dependency-weight aggregation
   pipeline (`aggregate_*_with_model` -> `f64`), a type graph entirely
   disjoint from `CapabilityId`/`DependencyEdge`/
   `CriticalityObservables.failure_blast_radius` (`u32`). No function
   bridges a `FailureDomainId` computation into `CriticalityObservables`.
   PATH-026 already established, for its own purpose, that identifier
   metadata is not authoritative evidence of independence; nothing here
   contradicts or needs to re-derive that. No PATH-035-E test was added
   (already-answered ground, per FIND policy of not duplicating settled
   results).

5. **Structural extent alone is insufficient for economic consequence
   (PATH-035-H, PATH-035-I).** Two single-capability failures with
   identical `effective_structural_branches` count (1 == 1) were
   constructed — one labeled (for human readability only, no code
   inference) as an optional reporting capability, one as a payment
   capability. The equal count cannot distinguish them; no economic
   weight is claimed or computed for either. Separately, one fixed
   two-branch topology was reused under two stipulated scenarios (total
   long-duration outage vs. partial short-duration degradation): the
   count is identical (2 == 2) because degradation degree and duration
   are not represented anywhere in `DependencyEdge`/`CapabilityId`
   topology — the graph is structurally blind to both dimensions.

## HYPOTHESIZED

- That `effective_structural_branches(source, edges).len()` (or some
  reduction built on it) is the *correct* structural-extent candidate
  for a future `Evidence -> FailureBlastRadius` derivation, as opposed to
  merely *one* candidate that happens to survive the specific attacks
  PATH-035 tried. No claim of completeness against all possible
  representation-laundering shapes is made.
- That a downstream reachable-node count computed some other way (e.g.
  restricted to a declared subset of "externally meaningful" capability
  nodes) could not also be made representation-invariant without reusing
  branch-counting. This alternative was not constructed or tested.

## NOT ESTABLISHED

- No claim is made about what `failure_blast_radius` values in any
  existing test or call site were *intended* to mean, since no
  derivation exists to consult.
- No economic weight, score, or severity ranking between capabilities is
  established or implied by PATH-035-H/I — only that count-of-one cannot
  distinguish them.
- No complete economic-consequence model (capacity loss, duration,
  recoverability, business criticality) is designed, scored, or wired in
  by this mission, per its explicit scope limits.
- `effective_structural_branches(...).len()` is **not** wired into
  `CriticalityObservables`, `measure_criticality`, `Contribution`, or any
  `aggregate_*`/allocation function. It remains an unwired candidate
  reduction, tested only against the attacks in `tests/pathological_cases.rs`
  (`path_035_*`).

## Outcome classification

**Outcome 3 (partial).** A structural branch-count reduction
(`effective_structural_branches(...).len()`) survives the node-splitting,
relay-insertion, duplication, ordering, and renaming attacks tried here,
using entirely pre-existing PATH-029/030 machinery — no new production
code was needed. It remains insufficient, by construction, for total
economic failure consequence (severity, duration, capacity degree), which
PATH-035 deliberately does not attempt to model.
