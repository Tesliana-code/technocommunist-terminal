# FIND-009 — SCC Condensation Erases Entry-Point Structure Inside Cyclic Convergence Regions

Status: OBSERVED
Protocol: PP-0.1-draft
Evidence: PATH-028-f, PATH-031, PATH-032

> **PATH-032 correction.** This finding originally claimed that
> single-entry and dual-entry cyclic convergence graphs were
> "topologically indistinguishable" and that "no topology-only algorithm
> could answer" which node independence was first lost at, requiring
> timestamps or causal metadata. **That claim was false and has been
> removed.** The two graphs in question have different edges and were
> never the same graph; what was actually true is narrower and is
> restated below. PATH-032 built the missing piece (entry-point topology)
> using nothing but existing graph structure, and it worked. The
> corrected version of this finding is below; nothing about the incorrect
> "add temporal metadata" conclusion should be relied upon or cited.

## Observation

PATH-028-f found that `first_structural_convergences` collapses to `[]`
when the shared-bottleneck set contains a cycle (`A->B->X, A->C->X,
X<->Y`): X and Y mutually disqualify each other under the "no other
shared node reaches me" rule, so no structural convergence is reported at
all — even though one plainly occurred.

PATH-031 fixed the mechanical collapse. It added
`cycle_aware_first_structural_convergences`, which groups the shared set
into strongly connected components of its *induced subgraph*, condenses
those into a DAG, and reports SCCs with no incoming edge from another SCC
as `StructuralConvergenceRegion`. On the PATH-028-f graph this correctly
returns one region `{X, Y}` instead of `[]`.

This is a solid mechanical result, unchanged by PATH-032: SCC condensation
of *any* directed graph is always a DAG, so "does another region reach
this one" can never itself re-encounter the mutual-disqualification bug.

PATH-031 also proved a clean structural fact worth recording on its own:
the shared-bottleneck set `S` is **downward-closed under reachability** —
if `u ∈ S` and `v` is reachable from `u` at all, `v ∈ S` too, because the
per-branch BFS that builds `S` always expands every non-source node it
visits. This means "induced subgraph over S" and "full-graph reachability
restricted to pairs in S" are provably the *same* partition for this
specific S. This still holds and is unaffected by the correction below.

## What PATH-031 actually found (the corrected claim)

PATH-031's `path_031_n` test compared two graphs:

**Single entry:** `A->B->X, A->C->X, X<->Y`. Both branches' *edges into
the region* land on X: `B->X` and `C->X`.

**Dual entry:** `A->B->X, A->C->Y, X<->Y`. The branches' edges into the
region land on *different* members: `B->X` and `C->Y`.

These are two **different graphs** — `C->X` vs. `C->Y` is a real,
observable difference in the edge set. `cycle_aware_first_structural_
convergences` returned the identical `members` set — `{X, Y}` — for
both, and PATH-031 mistook "these two different graphs produce the same
`members` output" for "these two graphs are indistinguishable by any
topology-only means." That inference does not follow: it only shows that
`members` (SCC membership) is a lossy summary of the graph — it does not
show the *graph itself*, or every function computable from it, must be
equally lossy.

## PATH-032: entry-point topology closes the gap

PATH-032 added `entries: Vec<StructuralConvergenceEntry>` to
`StructuralConvergenceRegion`. Each entry is `{ branch, member }`: which
branch (raw, or — preferred when composing with PATH-030 —
`effective_structural_branches`) reaches the region, and which specific
region member it reaches from outside the region first. This is computed
entirely from `DependencyEdge` topology already present in the input; no
new field, timestamp, or identifier was added to the data model.

Result, actual output:

```
single entry: {members: [X,Y], entries: [(B,X), (C,X)]}
dual entry:   {members: [X,Y], entries: [(B,X), (C,Y)]}
```

`members` is still identical. `entries` is not. `path_032_a` /
`path_032_b` assert this directly, including an explicit
`assert_ne!(entry_pairs(single), entry_pairs(dual))`. The distinction
PATH-031 called unrecoverable was recovered from the same input graph,
using only reachability and direct-edge lookups — no metadata added.

PATH-032 tested this across fourteen further shapes (`path_032_c`
through `path_032_p`) without finding a single counterexample:

- Longer relay paths into the same vs. different members (`c`, `d`) —
  entries correctly attribute to the branch **root** and the entered
  **member**, not to intermediate relay nodes; this is a deliberate,
  stated abstraction (the raw edges remain available if a caller ever
  needs the relay path), not a rediscovery of the original gap.
- One branch entering a region at two different members (`e`) — recorded
  as two entries sharing one branch, not merged and not double-counted as
  two branches.
- Duplicate edges (`f`), edge ordering (`l`), and renaming (`m`) are all
  inert on entries, exactly as they already were on members.
- A redundant/subsumed branch (`g`, reusing the PATH-029 absorption
  case) manufactures a spurious raw entry — and composing with
  `effective_structural_branches` removes it along with the branch,
  consistent with PATH-030's established composition recommendation.
- Three-member SCCs with identical `members` but three different entry
  patterns (all-via-X, via-X-and-Y, via-X-Y-Z) are all distinguished by
  `entries` alone (`i`).
- An upstream acyclic first convergence (`j`) and two separate first
  regions (`k`) show entries do not leak downstream or across regions.
- A cycle through the source (`n`) does not fabricate an entry, matching
  the existing source-as-dead-end convention used throughout PATH-027–031.
- Item O's adversarial search: two *different* graphs, same branch names,
  same region members, same coarse shape (one region, two members, three
  branches, three entries, occupied-member counts `{2,1}` either way) —
  `b,c->X, d->Y` versus `b->X, c,d->Y` — with the branches wired to
  members in the opposite 2:1 pattern. If a caller reduced a region to
  only "how many branches enter each member" (a coarser summary than
  `entries`), these two graphs would look identical. `entries` records
  full `(branch, member)` incidence rather than per-member counts, and
  does distinguish them (`c`'s entry member differs) — `path_032_o`.
  This is evidence *for* H3 within the tested family, not proof that no
  future graph shape could expose a gap.

## Finding

**CYCLIC CONVERGENCE MUST PRESERVE ENTRY STRUCTURE, NOT ONLY REGION
MEMBERSHIP.**

`StructuralConvergenceRegion.members` alone (an SCC's node set) discards
which branch enters through which member. That discarded information was
not causal or temporal — it was ordinary directed-edge topology
(`branch_root -> ... -> region_member`) that PATH-031's algorithm had
simply not been asked to retain. Once retained, it accounts for every
case PATH-031's own case-N counterexample and PATH-032's adversarial
suite (sixteen graph shapes, `path_032_a` through `path_032_p`) actually
tested.

## H3

**H3 (original graph topology + branch-specific entry edges into a first
shared SCC is sufficient to distinguish single-entry from dual-entry
cyclic convergence) SURVIVES THE CURRENT PATH-032 ADVERSARIAL SUITE.** It
successfully distinguishes all tested single-entry, dual-entry, and
multi-entry cases — including relay paths, repeated entries from one
branch, redundant-branch composition, three-member SCCs, and opposite
2:1 incidence patterns with identical coarse shape (item O) — using
existing topology alone. No counterexample was found *in the tested
structural family*. This is not a claim that no counterexample can
exist for some other graph shape not yet tried; it is a report of what
PATH-032's specific adversarial suite found and did not find.

## H1 vs. H2 (revised)

PATH-031's H1 (pure directed topology + SCC condensation is enough to
define cycle-aware convergence) is reaffirmed and, with PATH-032, is now
supported at both the region-membership level and the entry-point level.

PATH-031's H2 (a cyclic shared region proves the model lacks
causal/temporal information) is **retracted as stated**. What survives of
it is much narrower: SCC *membership by itself* does not encode
entry-point structure. That is a property of *one particular summary
function* (member-set reduction), not a property of the underlying graph
model. It was corrected, not merely qualified — the original phrasing
("no topology-only algorithm could answer it", "the input graph does not
contain the distinguishing information") was simply wrong and should not
be treated as having ever been an established result.

## Region representation

`StructuralConvergenceRegion { members: Vec<CapabilityId>, entries:
Vec<StructuralConvergenceEntry> }` — `members` unchanged from PATH-031;
`entries` added by PATH-032. Both are sets/lists, not a canonical single
node, for the same reason PATH-031 originally rejected a canonical
representative for `members`: every member and every entry is genuine
information, not a redundant duplicate (contrast PATH-029's canonical
representative, which was specifically for *redundant* subsumed
branches). Entries deliberately record `(branch root, region member)` and
not the full relay path between them — a chosen, stated level of
abstraction, not a loss, since the original `edges` remain available to
any caller that needs finer granularity later.

## Protocol consequence

`cycle_aware_first_structural_convergences` and its effective-branch-composed
counterpart are unchanged in name and signature; their return type is
richer, not different in kind. Neither is wired into
`aggregate_*_with_model`. A future aggregation model reducing a
multi-member, multi-entry region to a scalar still needs an explicit,
separately-justified reduction rule (count entries? count members?
weight by branch?) — but that is now an economic-design choice to make
deliberately, not a gap forced by missing information. PATH-032 removes
the previously-claimed prerequisite that a temporal/causal signal must be
added before this decision can even be considered.

## Next architectural experiment

- Decide the economic reduction rule for a `StructuralConvergenceRegion`
  with multiple members and/or multiple entries (this is a design
  decision now, not blocked on missing structural information).
- Re-run the PATH-030 composition comparison with entries included, to
  confirm the recommended pipeline (`effective_structural_branches` →
  `shared_structural_bottlenecks_from_effective_branches` →
  `cycle_aware_first_structural_convergences_from_effective_branches`)
  still holds once entries are part of what "composition" needs to
  preserve — PATH-032's `path_032_g` is a first data point (yes, it
  holds), not an exhaustive re-verification.
- If a genuinely new counterexample later shows entries insufficient for
  some other question (not the single-entry/dual-entry one closed here),
  document it with the same rigor PATH-032 used — a specific graph, not a
  restated intuition — before reaching for temporal or causal metadata
  again.
