# FIND-015 — Canonicalization Requires Explicit Representation Semantics

Status: OBSERVED / HYPOTHESIZED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-038

## Hypothesis under test (H9)

A deterministic topology-only canonicalization can remove representation-
granularity artifacts caused by pure relay / chain-decomposition nodes
while preserving meaningful branch, convergence, and target-transition
structure.

## OBSERVED

1. **No existing primitive performed node collapse or edge rewriting
   before this mission.** `effective_structural_branches` deduplicates
   branch ROOTS for counting; `first_structural_convergences`/
   `shared_structural_bottlenecks` compute facts about the graph;
   `structural_counterfactual`/`structural_propagation_surface` only add
   or remove edges via `remove_capability`. None of them fold a
   pass-through node's edges into a bypass edge.

2. **The tested contract (`canonicalize_pure_relays`): a node collapses
   only if, from topology alone,** it is not `source`; it is not in an
   explicit `protected` set; it has exactly one distinct predecessor and
   exactly one distinct successor; and its successor cannot reach it
   again (excludes cycle members). Collapse redirects predecessor to
   successor directly and repeats to a fixed point, logging every
   collapse in `RelayCollapseProvenance` (collapsed node + its
   predecessor/successor at the moment of collapse).

3. **The four purely structural guard conditions work exactly as
   specified, in isolation:**
   - A branch point (>= 2 successors) never collapses (`path_038_c`).
   - A convergence point (>= 2 predecessors) never collapses merely for
     having one successor (`path_038_d`).
   - A 3-cycle is left completely untouched — every member has exactly
     one predecessor/successor, but each also lies on the cycle, so none
     qualifies (`path_038_g`).
   - `source` is never collapsed even when it is itself topologically
     relay-shaped (`path_038_f`).
   - Duplicate edges, input edge order, and relabeling (modulo identity)
     do not affect the result (`path_038_h`/`i`/`j`).

4. **Simple relay and straight-chain decomposition are fully
   normalized.** `SOURCE->A->TARGET` and `SOURCE->A->RELAY->TARGET`
   canonicalize to the identical graph when `A` and `TARGET` are
   protected (`path_038_a`). A three-hop chain
   `SOURCE->A1->A2->TARGET` canonicalizes to `SOURCE->TARGET`, matching a
   direct baseline (`path_038_b`), and a multi-relay chain collapses to a
   fixed point in one call, idempotently (`path_038_l`). Every collapsed
   node is fully recoverable from provenance; nothing is silently deleted
   (`path_038_m`).

5. **Topology alone cannot certify that a degree-(1,1) node is
   representation-only rather than semantically meaningful
   (`path_038_n`).** `SOURCE->X->TARGET` is collapsed by the same rule
   regardless of whether `X` is stipulated as a true relay or as a
   genuinely meaningful capability that happens to sit in a 1-in/1-out
   position — the two situations are topologically IDENTICAL. The only
   way to prevent the false collapse is an explicit `protected`
   declaration, supplied externally, never derived from the graph.

6. **Explicit protection is necessary, but not sufficient
   (`path_038_k`/`path_038_p`) — the strongest result of this mission.**
   Given a genuine shared bottleneck (`SOURCE->X->M->PAYMENT,
   SOURCE->Y->M->PAYMENT`, already confirmed as a real convergence by the
   existing, previously-verified `shared_structural_bottlenecks`), with
   only `PAYMENT` protected: X and Y are each, INDIVIDUALLY, legitimate
   degree-(1,1) non-cyclic relay candidates — nothing is topologically
   wrong with collapsing either one alone. But collapsing both in
   sequence redirects both branches to the identical bypass edge
   `source -> m`, which deduplicates to one edge, dropping M's
   predecessor count from 2 to 1. M then becomes newly eligible and
   itself collapses, producing the single edge `source -> payment` —
   structurally indistinguishable from a graph that never had a shared
   bottleneck at all. **Protecting M too does not fix this**: M survives
   as a node, but its predecessor count has still silently dropped from
   2 to 1 in the canonical edges (`path_038_k`'s second assertion). The
   convergence FACT — that two distinct branches once fed M — becomes
   invisible in the canonical edges alone: `shared_structural_bottlenecks`
   run on the canonical graph reports nothing (`path_038_p`). It is not
   irrecoverably deleted (two distinct provenance entries record `m` as
   their `successor`, letting a careful reader reconstruct the original
   convergence), but the canonical edges BY THEMSELVES are not a safe
   substitute for convergence-sensitive structural analysis.

7. **The contract has an explicit scope limit distinct from the above:
   it does not cover leaf/terminal decomposition nodes
   (`path_038_o`).** For the pass-through relay shape from PATH-037-Q
   (`SOURCE->X->RELAY->PAYMENT`), canonicalizing before computing
   `structural_propagation_surface` makes the relay-inflated case agree
   exactly with the direct case, closing that specific cardinality
   inflation. But for the leaf-decomposition shape from PATH-037-Q2
   (`SOURCE->X->A, SOURCE->Y->A, A->A2`, where A2 has no outgoing edge at
   all), A2 is never eligible under this contract (it has zero
   successors, not one), so the Q2-style inflation is NOT resolved by
   this specific canonicalization. This is recorded as a scope limit, not
   hidden or worked around.

## HYPOTHESIZED

- That a broader canonicalization rule — one that also folds a
  zero-successor leaf node into its sole parent when the parent has
  exactly one child, under some additional protection/multiplicity
  discipline — could close the PATH-038-O leaf-decomposition gap without
  reintroducing the PATH-038-K/P cascading-collapse problem. Not
  designed or tested here.
- That a fundamentally different graph representation — one that carries
  explicit branch-multiplicity metadata on edges/nodes, so that
  deduplicating two bypass edges into one could be detected and refused
  — could avoid the PATH-038-K/P cascading collapse without abandoning
  edge deduplication elsewhere in this module (which FIND-007 already
  established as desirable). Not designed or tested here; this is the
  "different deterministic normalization" direction the original mission
  brief anticipated as a possible outcome.

## NOT ESTABLISHED

- That the canonical graph produced by `canonicalize_pure_relays` is
  economically correct, unique, or that topology reveals semantic
  capability identity. None of these is claimed anywhere in this
  mission.
- That protecting every node a caller cares about is sufficient to make
  canonicalization safe in general — `path_038_k` shows a case where
  protecting the one node that seemed to matter (M) still did not
  preserve the structural fact that mattered (its predecessor count).
- That degree-(1,1) means economically meaningless, in general or in any
  specific case. `path_038_n` establishes the opposite: topology cannot
  tell.
- That `structural_propagation_surface`'s scalar-count problems
  (PATH-037/FIND-014) are now solved. `path_038_o` shows canonicalization
  helps for the specific pass-through-relay shape PATH-037-Q attacked,
  but explicitly does not help for the leaf-decomposition shape
  PATH-037-Q2 attacked — the underlying question FIND-014 left open
  (whether a canonicalization step could remove ALL tested
  representation-granularity sensitivity) is answered NO for this
  specific, minimal contract, not resolved in general.
- `canonicalize_pure_relays` and `RelayCollapseProvenance` are **not**
  wired into `CriticalityObservables`, `measure_criticality`,
  `Contribution`, `structural_propagation_surface`, or any `aggregate_*`/
  allocation function.
- That `(canonical edges, provenance)` is a lossless representation of
  the original graph or of every structurally relevant fact. PATH-038-M/P
  establish only the specific recoverability properties tested there.

## Outcome classification

**Primarily Outcome 2, with an Outcome-3-flavored qualification found
during testing.** Canonicalization is only ever safe relative to an
explicit, externally supplied `protected` set — topology alone cannot
distinguish a true relay from a meaningful capability that happens to
share its shape (`path_038_n`, Outcome 2). But PATH-038-K/P go further
than a straightforward Outcome 2 result would predict: even WITH correct
protection of every node a caller explicitly cares about, the cascading
interaction of multiple independently-valid collapses can still silently
erase a real, previously-verified structural fact (a shared bottleneck's
predecessor count) from the canonical edges alone, recoverable only by
also consulting the full provenance log (Outcome 3's "unresolved
ambiguity / structural loss," surviving even with marking). H9 as
originally stated is REJECTED for "canonical edges as a safe drop-in
replacement for structural analysis"; canonical edges plus provenance
preserve enough information to recover the specific collapsed-node and
convergence facts explicitly tested in PATH-038-M/P, but general
losslessness is not established — which is a materially smaller claim
than what H9 proposed.
