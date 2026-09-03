# FIND-014 — Tested Scalar Blast-Radius Counts Lose Structural Information

Status: OBSERVED / HYPOTHESIZED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-037

> **EPISTEMIC CORRECTION.** The first version of this finding was titled
> "Failure Propagation Is a Vector, Not a Scalar Count" and claimed H8
> ("a single scalar ... cannot faithfully represent...") was "confirmed."
> That overstates what PATH-037 actually tested: PATH-037 attacked three
> concrete, count-like candidate reductions (A, B, C below) and one
> unreduced per-target output (D). All three counts failed under the
> attacks tried. That does **not** establish that no mathematically
> possible scalar encoding could ever preserve this information — a
> sufficiently elaborate scalar (e.g. an encoded/hashed representation of
> the whole transition vector) could theoretically carry arbitrary
> content, and that question is irrelevant to the economic-measurement
> question this mission is actually asking and was never tested. The
> corrected title, hypothesis, and outcome wording below scope every
> claim to the reductions actually tried. No experimental code or test
> was changed by this correction — only this finding's wording. See
> `docs/findings/FIND-013-*.md` for the analogous PATH-036 correction
> precedent this follows.

## Hypothesis under test (H8, corrected)

Original framing: "A single scalar 'blast radius' cannot faithfully
represent the structural consequences of capability removal when
different targets experience different kinds of change."

As actually tested:

**The tested count-like scalar blast-radius reductions (candidates A, B,
and C below) do not faithfully preserve the structural consequences of
capability removal when different targets undergo different transitions,
or when graph granularity changes (chain decomposition, relay
insertion).** No claim is made about the impossibility of every possible
scalar encoding — only that every count-like reduction actually tried
failed.

## OBSERVED

1. **No multi-target propagation primitive existed before this mission.**
   Inspection of `src/measurement.rs` found no production function
   enumerating reachable/downstream capabilities (PATH-035's
   `naive_downstream_reachable_count` was a test-only strawman) and no
   function producing per-target structural facts across more than one
   target at once — `structural_counterfactual` (PATH-036) answers
   exactly one `(source, target, removed)` query.

2. **Target universe chosen: `reachable_capabilities(source, edges)`
   (pre-removal), excluding `source` and `removed`.** Justified by:
   - **Monotonicity is provable and was tested
     (`path_037_inspection_removal_cannot_create_new_reachability`).**
     `remove_capability` only deletes edges; every edge usable after
     removal was already present before, so the post-removal reachable
     set is always a subset of the pre-removal one. A target unreachable
     before removal therefore can never usefully be included — it cannot
     have "gained" reachability, and confirming it stays unreachable adds
     no information.
   - `source` is excluded (trivially reachable from itself, zero
     informative content).
   - `removed` is excluded: its own loss of reachability is guaranteed by
     construction for any graph and reports nothing about propagation to
     OTHER capabilities (tested, `path_037_l`).
   - This is an edge-only representation: a capability with no edges at
     all cannot appear in this or any other structural primitive in this
     module, even if a larger declared-capability universe outside the
     graph considers it to exist. This limitation is inherited, not
     fixed.

3. **Four new deterministic, topology-only primitives were added**
   (`src/measurement.rs`, not wired into `CriticalityObservables`,
   `measure_criticality`, `Contribution`, or any `aggregate_*` function):
   `reachable_capabilities`, `StructuralPropagationEntry`,
   `structural_propagation_surface` (returns entries only for targets
   whose structural state changed, sorted lexically for determinism
   only). `structural_propagation_surface` is built entirely by calling
   `structural_counterfactual` once per candidate target — no traversal
   logic was duplicated.

4. **H8 survives for all scalar count reductions tested in PATH-037** —
   confirmed on two independent grounds, both scoped to the candidates
   actually attacked:
   - **Different targets can carry categorically different transition
     types in one removal event** (`path_037_b`, `path_037_e`,
     `path_037_p`): a hard reachability loss (`true -> false`) and a
     branch-count loss while remaining reachable (`true -> true`,
     branches down) can occur side by side. `path_037_p` constructs two
     scenarios with an IDENTICAL scalar affected-target count (2 in
     both) whose actual structural transitions are entirely different
     (`hard_loss_surface != branch_loss_surface` while
     `hard_loss_surface.len() == branch_loss_surface.len()`). This shows
     the specific count "number of affected targets" conceals a real
     distinction — not that every scalar must.
   - **Every count-like reduction tried in this mission is itself
     representation-sensitive**, independent of the transition-type
     problem above:
     - Candidate A ("targets losing reachability") and Candidate B
       ("targets with any structural change") both inflate 1 → 2 under
       relay insertion (`path_037_q`, restating `path_037_h`): a single
       stipulated pass-through node (RELAY) becomes its own changed
       entry with a transition shape *identical* to the target it
       precedes, adding no new information while inflating any count
       built on surface cardinality.
     - Candidate C ("count of target-scoped branch losses") also
       inflates 1 → 2 under chain decomposition (`path_037_q2`): a
       stipulated internal decomposition node (A2, reachable only
       through A) shows the identical branch-loss transition as A,
       inflating a branch-loss count that was hypothesized to be more
       robust to node-splitting than raw reachability counts. It was
       not.
     - Of the three count-like candidates evaluated (A, B, C), none
       survived a representation-granularity attack. This directly
       extends PATH-035/FIND-012's finding (raw affected-node count is
       not representation-invariant) to two further candidate counts
       that were not previously tested, in addition to reconfirming it
       for reachability-loss counts.
   - Candidate D (the full, unreduced per-target output / no scalar
     reduction) preserves the per-target transition facts represented by
     `StructuralPropagationEntry` — it does not collapse categorically
     different transitions into one number, so it does not exhibit the
     transition-type collapse from `path_037_p`. It is **not**
     representation-invariant in cardinality: relay insertion and chain
     decomposition can introduce additional target entries into it too
     (`path_037_h`/`path_037_q`/`path_037_q2` all show the surface's own
     length changing), so even the content the surface represents is
     tied to graph granularity. The narrower, supported claim is that D
     preserves the transition-type DISTINCTIONS that tested scalar
     counts A/B/C collapse — not that D is "representation-invariant in
     content" in any general sense.

5. **The PATH-036-D/L shared-bottleneck blind spot persists unchanged in
   the surface (`path_037_f`).** For the same queried target (PAYMENT),
   the surface's own entry is identical between a shared-bottleneck
   graph and a truly independent-paths graph — `first_structural_
   convergences` already distinguishes the two graphs, but
   `structural_propagation_surface` does not incorporate that
   information any more than `structural_counterfactual` did. The
   surfaces DO differ in overall length in this test (an extra entry for
   the shared bottleneck node M), but that is evidence of the SAME
   representation-sensitivity as item 4, not evidence that the surface
   principledly detects shared-bottleneck risk: M only appears because
   it happens to be modeled as its own labeled graph node in this
   representation.

6. **Cycle, source-removal, and empty/disconnected-graph behavior are all
   deterministic and non-panicking** (`path_037_k`, `path_037_m`,
   `path_037_n`, `path_037_o`). `removed == source` is not specially
   rejected: it is a well-defined, tested consequence of the same removal
   semantics (every one of source's outgoing edges is stripped, so every
   previously reachable target loses reachability) — not a separate
   contract.

7. **Duplicate-edge, relabeling, and edge-ordering invariance hold for
   the surface** (`path_037_g`, `path_037_i`, `path_037_j`), inherited
   directly from the underlying `structural_counterfactual`/
   `effective_structural_branches`/`is_reachable` machinery, with no new
   invariance logic required.

## HYPOTHESIZED

- That some other count-like reduction, not tried here (e.g. one that
  first collapses chain-decomposed/relay nodes via some canonicalization
  step), could survive the PATH-037-Q/Q2 attacks. No such reduction was
  constructed or tested.
- That a semantic capability-canonicalization step performed BEFORE
  reduction (i.e. before counting or scoring anything) could remove the
  representation-granularity sensitivity observed here, after which a
  scalar reduction of the canonicalized surface might behave
  differently. Not built or tested.
- That composing `structural_propagation_surface` with
  `first_structural_convergences`/`shared_structural_bottlenecks` per
  target could close the PATH-037-F blind spot. Not built or tested here.

## NOT ESTABLISHED

- **The impossibility of every possible scalar encoding of the
  propagation surface.** PATH-037 tested three specific count-like
  reductions (A, B, C); it did not attempt, and this finding does not
  claim to have ruled out, more elaborate scalar encodings (e.g. a
  packed/hashed representation of the full transition vector, or a
  scalar computed after some normalization step). Only the tested
  candidates are shown to fail.
- **Whether a canonical representation-normalization step exists** that
  could collapse chain-decomposed/relay-inserted nodes back to one
  logical capability before any counting happens, and if so, whether a
  scalar reduction applied AFTER such normalization would survive the
  PATH-037-H/Q/Q2 attacks. Neither the existence nor the nonexistence of
  such a step is established here.
- No operational degradation degree, capacity, latency, duration, or
  economic weight is computed, scored, or implied anywhere in this
  mission.
- No claim that unreachable is worse than branch-loss economically, that
  more affected targets means more critical, that a target count is
  severity, or any monetary consequence.
- Which (if any) of candidates A/B/C/D is the adopted meaning for
  `failure_blast_radius` is NOT decided; `failure_blast_radius` still has
  no adopted deterministic derivation (FIND-012 stands unchanged).
- `reachable_capabilities`, `StructuralPropagationEntry`, and
  `structural_propagation_surface` are **not** wired into
  `CriticalityObservables`, `measure_criticality`, `Contribution`, or any
  `aggregate_*`/allocation function.

## Outcome classification

**Outcome 2 (H8, as tested, survives).** Existing/extended machinery
enumerates a deterministic per-target structural transition vector
(`StructuralPropagationEntry`/`structural_propagation_surface`). PATH-037
found no tested count-like scalar reduction (candidates A, B, or C) that
preserved both the transition-type distinctions (`path_037_p`) and the
representation-granularity behavior attacked here
(`path_037_h`/`path_037_q`/`path_037_q2`) — including two candidates
(B and C) not previously tested by PATH-035, extending that mission's
finding about raw node counts. The unreduced per-target surface
(candidate D) preserves the transition-type distinctions the tested
scalars lose, but its own cardinality remains sensitive to graph
granularity in the same way. Whether some untested, more elaborate scalar
encoding — or a scalar computed after a canonicalization step — could
avoid both failure modes is explicitly not established.
