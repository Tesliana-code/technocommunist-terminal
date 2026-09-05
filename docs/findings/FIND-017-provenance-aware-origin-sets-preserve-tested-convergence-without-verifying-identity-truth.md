# FIND-017 — Provenance-Aware Origin Sets Preserve Tested Convergence Without Verifying Identity Truth

Status: OBSERVED / HYPOTHESIZED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-040

## Hypothesis under test (H11)

A provenance-aware or multiplicity-aware canonical representation can
preserve genuine pre-collapse convergence through canonicalization while
remaining invariant to artificial duplicate edges.

## Background

FIND-016's strongest counterexample (`path_039_o`) showed that even fully
truthful, individually-correct `RepresentationOnly` declarations for X and Y
in

    source -> X -> M
    source -> Y -> M

still erase M's shared-bottleneck status: both bypass edges deduplicate onto
`source -> M`, and `shared_structural_bottlenecks` on the canonical graph
reports nothing. This is independent of declaration truth; it is a
consequence of how bypass edges deduplicate.

## Candidates tested

**Candidate A — raw multiplicity.** A literal count of `(from, to)`
occurrences in the raw edge list, with no set-based structural dedup.

**Candidate B — distinct structural origins.** For each canonical edge,
replay the REAL `RelayCollapseProvenance` list already returned by the
unmodified `canonicalize_declared_representation_nodes` (PATH-039) as a
union-of-sets: every raw edge seeds a singleton origin set (`HashSet`
insert, not a count), and each collapse event unions the origin sets of
the two edges it consumes into the origin set of the edge it produces.
`path_040_distinct_origin_branch_count(source, origin_set)` then counts
how many of `source`'s distinct immediate branches are represented in one
edge's origin set.

Candidate C (richer path provenance) was **not tested** — Candidate B did
not produce a counterexample requiring it (see Stopping Condition).

Both candidates are test-local (`tests/pathological_cases.rs`), not part of
`src/measurement.rs`. `canonicalize_pure_relays`,
`canonicalize_declared_representation_nodes`, and `shared_structural_bottlenecks`
are unmodified.

## OBSERVED

1. **Candidate A is rejected exactly as expected**
   (`path_040_candidate_a_raw_multiplicity_is_inflated_by_a_duplicate_raw_edge`).
   One genuine edge serialized twice reports raw multiplicity 2, not 1 —
   the FIND-007/PATH-017 pathology reproduced at the edge level.

2. **Candidate B reproduces `path_039_o`'s exact erasure first, then
   recovers what was erased** (`path_040_a`). `shared_structural_bottlenecks`
   on the canonical graph is confirmed empty (the erasure is real), while
   the origin set for `source -> m` contains both `x`'s and `y`'s
   contributing raw edges, giving a distinct-origin-branch count of 2. A
   derived predicate over `origins` alone (no access to the original edge
   list at the call site) correctly re-identifies `m` as the sole
   bottleneck — the exact fact `shared_structural_bottlenecks` lost.

3. **Candidate B rejects the duplicate-raw-edge attack** (`path_040_b`). A
   single genuine branch (`source -> x -> target`) with every raw edge
   duplicated in the input list still yields exactly one provenance entry
   (the production collapse loop already dedups raw edges into a
   `HashSet` before any collapse runs) and exactly one origin branch —
   consistent with FIND-007.

4. **Candidate B survives independent multi-hop branches**
   (`path_040_c`). Two branches, each two representation-hops deep
   (`source -> x1 -> x2 -> m`, `source -> y1 -> y2 -> m`), still converge
   on a distinct-origin-branch count of 2 after full collapse to
   `source -> m`.

5. **Candidate B survives cascading collapse** (`path_040_d`). A branch
   requiring three successive collapse stages (`a -> b -> c` all
   representation-only) converging with a one-hop branch `y` still yields
   exactly 2 origins at `m` — neither inflated to 3 (one per stage) nor
   erased to 0 by later stages overwriting earlier provenance. This
   directly tests the union-of-origin-sets bookkeeping across multiple
   fixed-point iterations of the same shared `collapse_relays_where` core.

6. **The no-convergence control produces no manufactured provenance**
   (`path_040_e`). A plain single chain reports exactly 1 origin branch;
   `shared_structural_bottlenecks` remains empty as expected.

7. **Candidate B is invariant to relabeling, edge order, and harmless
   relay depth** (`path_040_f`). Renaming every representation node to an
   arbitrary label, reversing edge input order, and inserting an extra
   truthful representation relay into one branch still yields a
   distinct-origin-branch count of 2 for the (renamed) convergence node.

8. **Candidate B requires no semantic truth beyond what `path_039_o`
   already assumes.** It operates purely on topology and the existing
   `RelayCollapseProvenance` output; no issuer identity, authority, or
   attestation concept is introduced.

## HYPOTHESIZED

- That a compressed form of Candidate B — storing only a per-canonical-edge
  set of *branch-root identities* (the statistic actually consumed by
  `path_040_distinct_origin_branch_count`) rather than the full set of raw
  contributing edges — would preserve every tested invariant while using
  materially less storage than the full origin-edge sets built here. Not
  implemented or tested; the tested implementation retains full origin-edge
  sets, which is more information than the tested invariant requires.
- That composing Candidate B with the representation-claim layer from
  PATH-039 (i.e., only trusting an origin's distinctness when its branch
  root also carries an `Authorized` `RepresentationOnly` claim) would
  narrow, but not close, the identity-truth gap in point 2 of NOT
  ESTABLISHED below. Not designed or tested.

## NOT ESTABLISHED

- **Whether distinct node identity means distinct real-world dependency.**
  Candidate B's origin count is exactly as trustworthy as the assumption
  that two differently-named nodes are two genuinely different things.
  Nothing tested here (or in PATH-039) verifies this. An attacker (or a
  data-entry error) that names one real dependency `x` in one place and
  `x_prime` in another produces the identical structural signature as two
  genuinely independent branches — this is the same laundering surface
  FIND-016 already identified (`path_039_d`/`l`), inherited unchanged, not
  newly introduced or newly resisted by Candidate B. See "Strongest
  counterexample" below.
- **Whether Candidate B's full origin-edge-set representation is
  necessary**, or whether the tested invariants (bottleneck recovery,
  duplicate-edge resistance, cascading-collapse survival, relabeling
  invariance) require only the smaller branch-root-identity summary
  described in HYPOTHESIZED. The tested implementation retains, in
  aggregate across all canonical edges, exactly the original edge set
  (every raw edge is stored in exactly one edge's origin set) — so as
  tested, Candidate B does **not** shrink the total provenance data below
  the size of the original graph. Whether a compressed form suffices is
  HYPOTHESIZED, not established.
- **Whether Candidate B is economically meaningful.** Not wired into
  `Contribution`, `CriticalityObservables`, `measure_criticality`, any
  `aggregate_*` function, or allocation. No claim is made that "2 distinct
  origins" implies twice the importance of "1 origin."
- **Whether collapse order could ever change the result for graphs more
  complex than those tested here.** All PATH-040 cases confirmed order-
  independence (union is commutative/associative, and
  `collapse_relays_where`'s candidate selection is already lexically
  deterministic), but this was not exhaustively fuzzed against arbitrary
  topologies.
- Whether Candidate C (richer path-shaped provenance, not just an origin
  set) is ever required. Not tested; Candidate B produced no counterexample
  that would have required it.

## Strongest counterexample

Candidate B cannot distinguish two genuinely independent branches from two
differently-labeled branches describing the same real dependency. Consider:

    source -> x -> m      (claimed RepresentationOnly, truthfully independent)
    source -> y -> m      (claimed RepresentationOnly, truthfully independent)

against a structurally identical graph where `x` and `y` are, in reality,
two accidental (or adversarial) labels for the *same* underlying
capability, split so that each half looks like an independent
representation chain. Both graphs produce the exact same canonical edge
(`source -> m`) and the exact same Candidate B origin set (two distinct
`(from, to)` origin entries, distinct-origin-branch count 2). Candidate B
would report identical provenance for two graphs that — per the mission's
own adversarial question — "require different bottleneck conclusions." This
is not a new flaw specific to Candidate B's bookkeeping; it is FIND-016's
already-established identity-truth gap (`path_039_d`/`l`), which no
topology-only or provenance-only mechanism can close without an
issuer-identity or attestation concept this mission is scoped not to
invent.

## Outcome classification

**H11 CONFIRMED, narrowly, for the specific invariant tested.** Candidate
B — a `HashSet`-seeded union of pre-collapse raw edges, replayed over the
existing `RelayCollapseProvenance` output — simultaneously satisfies all
five success conditions against the required PATH-040-A through F cases:
it preserves the exact convergence `path_039_o` erased (2); it rejects the
duplicate-raw-edge attack (3); it survives a 3-stage cascading collapse
without inflation or erasure (5); it is invariant to relabeling, edge
order, and harmless additional relay depth (7); and it requires no
semantic-truth mechanism beyond what PATH-039 already assumes (8).

The claim does **not** generalize beyond what was tested: distinct-origin
counting answers "how many structurally distinct pre-collapse edges fed
this canonical edge, given the node identities as labeled" — not "how many
genuinely distinct real-world dependencies fed this node." The strongest
counterexample shows that gap is real and is inherited unchanged from
FIND-016, not closed by this mission. As tested, Candidate B also does not
compress the provenance data below the size of the original edge set — it
redistributes the original edges into per-canonical-edge buckets rather
than discarding any of them, so "does this require retaining the entire
original graph" is answered **yes, as implemented here**, with a
compressed alternative left HYPOTHESIZED rather than built.
