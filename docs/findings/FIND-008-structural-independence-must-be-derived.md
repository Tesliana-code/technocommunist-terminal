# FIND-008 — Structural Independence Must Be Derived From Topology

Status: OBSERVED (partial — see Unresolved Semantics)
Protocol: PP-0.1-draft
Evidence: PATH-027, PATH-028, PATH-029, PATH-030, PATH-031, PATH-032

## Observation

PATH-027 established `shared_structural_bottlenecks(source, edges)`: a node is
reported when it is reachable from at least two of `source`'s distinct
structural branches. This correctly rejected label-based independence
laundering (`DependencyPathId`, `FailureDomainId`) and simple indegree
counting.

It also produced a graph shape PATH-027 could not itself resolve:

```
A -> B -> X
A -> C -> X
          X -> D
```

`shared_structural_bottlenecks` returns `[D, X]`. Both are "correct" under
its own rule — D remains reachable from both of A's original branches — but
this leaves an open question: is D a *second* structural loss of
independence, or a continuation of the *same one* that already happened at
X?

PATH-028 extended the graph and made the risk concrete:

```
A -> B -> X -> D -> E -> F
A -> C -> X
```

`shared_structural_bottlenecks` returns `[D, E, F, X]` — four nodes — from a
single convergence event at X. A long shared tail after one real bottleneck
inflates the reported count of "shared" nodes without bound.

## Finding

Distinguish four separate structural concepts that PATH-024–027 evidence
had left conflated in earlier discussion:

- **declared identity** — `DependencyId` / `DependencyPathId` /
  `FailureDomainId`. Already shown (PATH-019, PATH-024, PATH-026) not to be
  authoritative evidence of independence; can be relabeled freely without
  changing topology.
- **structural edges** — `DependencyEdge { source, target }`. The only
  input `shared_structural_bottlenecks` and `first_structural_convergences`
  accept. Carries no identity metadata at all.
- **branch independence** — whether two of `source`'s immediate out-edges
  lead to genuinely disjoint downstream reachable sets (PATH-027-a). PATH-029
  found that "immediate out-edge" and "independent branch" are not the same
  thing — a branch whose entire reach is gatekept by another branch
  (`effective_structural_branches`, below) is not independent even though it
  is a distinct immediate edge.
- **convergence vs. downstream continuation** — once two branches meet at a
  node, everything reachable from that node onward is reachable from both
  branches by construction. That is one structural event (loss of
  independence at the meeting point), not N events.

PATH-028 built a second primitive, `first_structural_convergences`, that
reports only nodes in the shared set that are not themselves reachable from
another shared node. On the long-tail graph above it returns `[X]`, not
`[D, E, F, X]`.

```
PATH-028 downstream convergence
shared-reachability = ["d", "e", "f", "x"]
first-convergence   = ["x"]
```

This supports the candidate invariant PATH-028 was scoped to test:

**ONE STRUCTURAL LOSS OF INDEPENDENCE MUST NOT MULTIPLY ECONOMIC IMPORTANCE
MERELY BECAUSE THE SHARED PATH CONTINUES DOWNSTREAM.**

`shared_structural_bottlenecks` violates it (by design — that was never its
job); `first_structural_convergences` satisfies it on every graph tested
where the shared set has a well-defined minimal frontier (PATH-028 a–e, h,
i).

## Unresolved semantics (do not treat as settled)

Two adversarial cases attacked `first_structural_convergences` itself and
found real gaps, not implementation bugs:

1. **Cyclic shared nodes (PATH-028-f) — RESOLVED by PATH-031 + PATH-032.**
   If the shared bottlenecks themselves form a cycle downstream of the
   real convergence point — `A->B->X, A->C->X, X<->Y` — the "no other
   shared node reaches me" rule makes X and Y disqualify each other.
   `first_structural_convergences` still returns `[]` on this graph (that
   primitive is unchanged). PATH-031 added
   `cycle_aware_first_structural_convergences`, which condenses the
   shared set's induced subgraph into SCCs and reports each
   no-incoming-edge SCC as a `StructuralConvergenceRegion` — on this
   graph, one region `{X, Y}`, not `[]`. SCC condensation of any directed
   graph is always a DAG, so the region-level "does another region reach
   this one" check cannot itself re-encounter the mutual-disqualification
   bug.

   PATH-031 additionally raised, but PATH-032 corrected, a claim that a
   *further* piece of information (which specific member each branch
   entered the cycle through — `A->B->X, A->C->X` vs. `A->B->X, A->C->Y`,
   both against `X<->Y`) was lost and unrecoverable without adding
   timestamps or causal metadata. That claim was wrong: the two graphs
   differ in ordinary topology (`C->X` vs. `C->Y`), and PATH-032 recovered
   the distinction by adding `entries: Vec<StructuralConvergenceEntry>`
   to `StructuralConvergenceRegion` — `(branch, region member)` pairs
   computed from the same input edges, no new data added. See
   **FIND-009** (revised) for the corrected account; the finding's
   original "no topology-only algorithm could answer it" language has
   been retracted there, not merely softened.

2. **Branch-root absorption (PATH-028-g) — RESOLVED by PATH-029.** If one
   branch flows directly into another branch's root — `A->B->C, A->C->D`
   (B feeds C, C is also A's own second branch) — PATH-029 built
   `effective_structural_branches(source, edges)` and found the answer is
   **no, B does not count as an independent branch**: `B`'s entire
   reachable set beyond itself is gatekept by `C` (delete `C` and `B`
   reaches nothing), while `C`'s reach does not depend on `B` at all.
   `effective_structural_branches` returns `[C]`, not `[B, C]`.

   This is *not* plain reachable-set containment. On this exact graph,
   `reachable(C) = {C, D}` is a subset of `reachable(B) = {B, C, D}` — the
   naive "smaller set is subsumed" rule flags `C` (which needs nothing)
   as subsumed by `B` (which needs everything), exactly backwards. The
   correct test is counterfactual removal: `r1` is subsumed by `r2` iff
   `r2` is reachable from `r1` and deleting `r2` leaves `r1` unable to
   reach anything but itself. This also correctly preserves genuine
   partial independence — a branch that reaches into another branch's
   territory but *also* does unique work of its own (e.g. `A->B->X,
   A->C->X, B->Y`) is not subsumed, because `Y` survives `C`'s removal.

   PATH-029 also found that subsumption among three or more branch roots
   can form a one-directional cycle (`B` subsumed by `C`, `C` by `E`, `E`
   by `B`, with no single mutual pair) that, without transitive-closure
   handling, collapses all three branches to an empty result. This
   **is** treated as a genuine defect (not an accepted ambiguity, unlike
   item 1 above) and is fixed inside `effective_structural_branches` by a
   small transitive closure over the subsumption relation *among branch
   roots only*, grouping mutually-reaching branch roots into one
   equivalence class and keeping a single deterministic representative.
   This is a narrowly scoped SCC condensation of a small derived relation,
   not a general graph-SCC library over the whole capability graph.

Item 1's cyclic shared-*bottleneck* problem is a different relation from
item 2's branch-root subsumption cycles (reachability among shared
bottlenecks, not among branch roots), so PATH-029's SCC handling above
does not, by itself, correct it — that took PATH-031's separate SCC
condensation over the shared-bottleneck set (see item 1). Per the
research program's own instruction, the smallest counterexample was
recorded instead of hidden behind added complexity, until PATH-031/032
resolved it directly.

## PATH-030: composing effective branches with convergence analysis

PATH-029 left open whether `effective_structural_branches` should feed
into the convergence primitives. PATH-030 added two composed candidates
(`shared_structural_bottlenecks_from_effective_branches`,
`first_structural_convergences_from_effective_branches`) without mutating
the existing three primitives, and tested composition against seven graph
shapes plus a renaming-sensitivity check.

**Composition removes real artifacts, not real convergence.** Two cases
demonstrate a genuine bug in the raw (branch-unaware) analysis:

- `A->B->C, A->C->D`: raw `shared_structural_bottlenecks` reports
  `[C, D]` — C is flagged "shared" purely because the subsumed branch B
  *also* reaches it, not because two independent branches converge there.
  With `B` correctly excluded (PATH-029), composed analysis reports `[]`:
  there was only ever one real branch, so nothing converges.
- `A->B, A->C, A->E, B->C, E->F`: raw analysis reports `C` as shared for
  the same reason (B's subsumed presence). Composed analysis reports `[]`.
  This is the "artifact caused solely by [the subsumed branch]'s presence"
  PATH-029's report predicted PATH-030 should check for — confirmed.

**Composition does not remove genuine convergence.** Three cases where the
branches are not subsumption-related (a plain diamond; a long shared tail
after a real 2-branch convergence at X; a partial-overlap branch that does
unique work before also reaching the other branch's territory) all produce
*identical* raw and composed results. Effective-branch filtering is a
no-op when there is nothing to normalize.

**Branch-root SCCs collapse convergence to empty, and that is provably not
a loss of information.** `A->B, A->C, B->C, C->B` (and its 3-node
one-directional-cycle analogue) reduce to one effective branch, and
composed shared/first-convergence both become `[]`, where raw analysis
reported `[B, C]` shared / `[]` first (its own PATH-028-f-style
ambiguity). This is correct, not lossy: nodes in the same SCC are
mutually reachable, so by definition they have *identical* reachable sets
outside the group — collapsing to one representative cannot hide a
downstream node one branch could reach and the other could not. What
disappears is the false appearance of "B and C converging with each
other", which was never a real convergence with anything outside the
group.

**Renaming sensitivity (item 5 of the PATH-030 brief).** Two
structurally-identical 2-cycles labeled differently (`{b,c}` vs. `{p,q}`)
select different canonical representatives (`b` vs. `p` — the
lexicographically smallest label in each case) but produce
identically-*shaped* composed results (same branch count, same composed
result count, both empty in this case). The specific label returned is
metadata — an arbitrary but deterministic tie-break among structurally
equivalent nodes — not a structural/economic output. This is consistent
with, not a violation of, INDEPENDENCE IS A PROPERTY OF GRAPH STRUCTURE,
NOT OF LABELS: the *label* selected is naming-sensitive by construction
(it has to pick one of several equally-valid identical nodes), but the
*structure* of the answer is not.

**PATH-028-f is not resolved by composition, and the two problems are
independent.** `A->B->X, A->C->X, X<->Y` has no subsumption relation
between the branch roots B and C (neither reaches the other), so
`effective_structural_branches` returns `[B, C]` unchanged, and the
composed results are byte-identical to the raw ones — including the
unresolved ambiguity (`first_structural_convergences` still collapses to
`[]`). Branch-root normalization (PATH-029) and convergence-cycle
semantics (PATH-028-f) are separate problems operating on different
relations; fixing one does not touch the other.

## Principle

STRUCTURAL CONVERGENCE IS A PROPERTY OF WHERE BRANCHES FIRST MEET, NOT OF
HOW LONG THE SHARED PATH CONTINUES AFTERWARD. Cyclic shared regions are
now covered too: PATH-031's `cycle_aware_first_structural_convergences`
reports a cyclic first-convergence as one `StructuralConvergenceRegion`
rather than dropping it, and PATH-032 preserves which branch enters
through which region member. See FIND-009 for the cyclic-region account.

## Protocol consequence

No composition of these primitives is adopted as *the* PP-0.1
economic-importance signal by this finding — that still requires an
explicit economic-reduction decision (see "Next architectural
experiment" below), not further structural resolution: PATH-028-f itself
is now resolved (PATH-031/032). But PATH-030 does narrow the architecture
down from "two independent axes with four unclear combinations" to a
specific recommended pipeline:

**AXIS 1 (which branches are genuinely independent) should run first, and
its output should feed AXIS 2 (where independent branches first lose
independence).** Concretely: `effective_structural_branches` →
`shared_structural_bottlenecks_from_effective_branches` /
`first_structural_convergences_from_effective_branches`, not the raw
(`source`-immediate-edges) versions. PATH-030's adversarial evidence is
one-directional in support of this: every case where raw and composed
diverged, the composed result was the one without a demonstrable defect
(a subsumed branch manufacturing a fake shared node, or an SCC's internal
symmetry manufacturing a fake mutual-convergence pair); every case where
raw and composed agreed, they agreed exactly. No adversarial case found
composition destroying a convergence signal that raw analysis got right.

All five functions remain available in `src/measurement.rs` for
inspection and comparison — this finding recommends the composed pipeline
as the better-evidenced default, it does not delete or hide the raw
primitives, and it explicitly does not yet wire any of them into
`aggregate_*_with_model`.

## Next architectural experiment

- PATH-028-f's collapse is resolved (PATH-031,
  `cycle_aware_first_structural_convergences`), and its entry-point
  provenance is preserved (PATH-032, `entries` on
  `StructuralConvergenceRegion`) — see FIND-009 (revised). A
  temporal/causal signal is **not** the immediate next decision: PATH-032
  showed the previously-claimed missing information was recoverable from
  existing topology, so no such signal is currently blocking anything.
- The next decision is a design choice, not a research gap: how should a
  future aggregation model reduce a `StructuralConvergenceRegion` (with
  its `members` and `entries`) to a single economic-importance
  contribution — count of members? count of distinct entering branches?
  something else? This is deliberately left open here rather than decided
  by default.
- Only after that should a graph-derived structural-independence primitive
  (the composed pipeline: `effective_structural_branches` →
  `shared_structural_bottlenecks_from_effective_branches` →
  `cycle_aware_first_structural_convergences_from_effective_branches`) be
  wired into `aggregate_*_with_model` as an economic-importance input.
