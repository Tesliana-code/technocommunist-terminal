# PKV PATH/FIND Index 001

Compact index of `docs/findings/` and `docs/research/`. Built from document
headers and Evidence/Status lines only; no inferred history.

| PATH | FIND | Title | One-sentence result | Canonical prior commit | Status |
|---|---|---|---|---|---|
| 010, 011 | 001 | Capital Pool Remainder Must Remain Explicit | Capital Pool can exceed verified claims, so an explicit Capital Remainder term is required. | — | observed |
| 004, 008, 009 | 002 | Stewardship Must Be a Separate Economic Claim | Including Stewardship in the Contribution formula while also paying it via a Stewardship Pool double-counts it (~13.89% inflation). | — | observed |
| (none) | 003 | Manual Scores Are Not Economic Evidence | Normalized manual scores must be backed by definitions, observables, a deterministic function, evidence, and versioned normalization. | — | design correction |
| 012 | 004 | Raw Counts Are Gameable Observables | Saturating raw dependency count still yields nonzero Criticality for an artifact with no real risk signal. | — | observed |
| 015 | 005 | Weak Signals Must Not Accumulate Into False Systemic Importance | A `1 - product(1-x)` aggregator lets many weak dependencies accumulate into false high importance. | — | observed |
| 016 | 006 | Threshold Cliffs Amplify Small Evidence Changes | A 0.20 hard threshold turns a 0.002 evidence delta into a 0.201 output jump. | — | observed |
| 017 | 007 | Duplicate Edges Must Not Create Economic Importance | RankedDecay aggregation is not duplicate-resistant: one dependency replicated 100x saturates to 1.0. | — | observed |
| 027–032 | 008 | Structural Independence Must Be Derived From Topology | `shared_structural_bottlenecks` correctly rejects label-based laundering but leaves an unresolved case (single vs. repeated loss of independence at a convergence node). | — | observed (partial) |
| 028-f, 031, 032 | 009 | SCC Condensation Erases Entry-Point Structure Inside Cyclic Convergence Regions | Cycle-aware SCC condensation (PATH-031/032) fixes the mechanical collapse of convergence detection inside cycles; document contains a correction removing an earlier false "topologically indistinguishable" claim. | — | observed |
| 033 | 010 | Distinct Entering-Branch Count Is a Partial, Not Sufficient, Economic Reduction | Distinct-branch count resists SCC-size inflation but is only a partial reduction of convergence-region economic weight (H4 partially held). | — | observed |
| 034 | 011 | Existing Substitutability Semantics Do Not Explicitly Preserve Restoration Time | No existing field/function captures restoration-time semantics; supersedes an overclaiming earlier draft. | — | observed / hypothesized (mixed) |
| 035 | 012 | Failure Blast Radius Lacks a Representation-Invariant Derivation | `failure_blast_radius` has no derivation at all (always a literal input), and naive node counting is representation-gameable (H6 confirmed against a strawman). | — | observed / hypothesized / not established (mixed) |
| 036 | 013 | Structural Counterfactuals Do Not Determine Operational Degradation | Contains a self-flagged correction: original branch-count fields leaked source-global (not target-scoped) data; fixed and re-verified. | — | mixed |
| 037 | 014 | Tested Scalar Blast-Radius Counts Lose Structural Information | Corrects an overclaimed title/hypothesis; three tested scalar reductions all lose structural information, but "no scalar could ever work" was not established. | — | mixed |
| 038 | 015 | Canonicalization Requires Explicit Representation Semantics | `canonicalize_pure_relays` is the first topology-only node-collapse primitive, gated by strict predecessor/successor/cycle conditions. | — | mixed |
| 039 | 016 | Explicit Representation Roles Reduce Ambiguity but Create a New Laundering Surface | New representation-role types authorize canonicalization but create a new declaration-laundering surface (`path_039_o`). | — | mixed |
| 040 | 017 | Provenance-Aware Origin Sets Preserve Tested Convergence Without Verifying Identity Truth | Provenance/multiplicity-aware representation preserves pre-collapse convergence but cannot verify identity truth. | — | mixed |
| 041 | 018 | Structural Corroboration Raises Laundering Cost but Cannot Establish Independence Truth | Isomorphic true-independence and split-label-laundering graphs are provably indistinguishable by topology alone. | — | mixed |
| 042 | 019 | Identity Truth Requires at Least One Exogenous Trust Axiom | An independent-observer-identity primitive (Candidate B) breaks some identity ties FIND-018 showed topology cannot. | — | mixed |
| 043 | 020 | Bounded + Epistemic-State Measurement Preserves Structure Without False Precision | A bounded/epistemic-state measurement layer can honestly report SUPPORTED/UNRESOLVED/CONFLICTED without fabricating precision. | — | mixed |
| 044 | 021 | Bounds + State Are Sufficient for Scalar Criticality Queries, Not for Explanation | `measure_criticality` is separable and monotonic in its four observables, which is sufficient for bounded scalar queries but not for explanation. | — | mixed |
| 045 | 022 | All-Worlds Dominance Is a Sound Partial Order, But Requires Joint Admissible State | No production function currently ranks capabilities/contributors against each other; all-worlds dominance is sound but needs joint admissible state to apply. | — | mixed |
| 046 | 023 | Joint Admissible States Are Necessary for Honest Normalization, and Production Has Real, Confirmed Gaps | Confirms real production gaps (NaN not caught, no dedup of duplicate `ContributionEvent`s, no output-order guarantee) in addition to the joint-admissible-state theory. | — | mixed |
| 046.5 | 024 | `ContributionEvent` Has No Authoritative Identity: Replay Protection Is a CASE 3 Blocker | Bounded production remediation: fixed the non-finite-value defect (NaN/inf validation); confirmed replay/duplicate-event protection remains unresolved (CASE 3 blocker). | — | mixed |
| 047 | 025 | Source-Scoped Event Identity Requires Three Independently-Verifiable Exogenous Facts, Not One Combined Trust Label | `(source_id, event_id)` identity resolution needs AUTHORITY plus SEPARATION or STABILITY as independent facts, not one combined label; document records two prior model defects fixed under cross-review. | — | mixed |
| 048 | 026 | Durable Cross-Call Identity Memory Is Necessary for Replay Safety, and Degrades to `Unresolved`, Never to a Silent Guess, at Every Limit Tested | Durable cross-call state is necessary (Temporal Identifiability Theorem) and safe only under an explicit, itself-unverifiable `HistoryCompleteness` attestation; a real resolver bug found and fixed (regression case PATH-048-K). | — | mixed |
| 049 | 027 | Durable Replay Memory Requires an Atomic Occurrence-Claim Boundary Under Concurrency | An atomic, single-winner `Reserved(K)` insertion is required under concurrency; corrects an earlier overclaim that canonicalization must execute inside the lock. | PATH-048 at `27ff331` | mixed |
| 050 | 028 | Reserved State Alone Cannot Safely Recover an External Economic Effect After an Independent Crash Boundary | Safe recovery requires either an authoritative completion witness or harmless-retry effect semantics; a bare idempotency key is not itself such a witness. | PATH-049 at `3d38c74` | mixed |
| 051 | 029 | Absence of Provider Completion Memory Does Not Establish Absence of Economic Effect Across an Independent Persistence Gap | Provider-side receipt/idempotency memory can itself lag the real effect, so its absence after a crash is not proof the effect didn't happen (P0/P1 identical recovery-visible state, divergent ground truth). | PATH-050 at `da3afb5` | mixed |

Research notes indexed:

| Note | Scope | Status |
|---|---|---|
| `PKV-TRANSACTION-RECOVERY-MAP-001.md` | Maps FIND-025..029's already-tested results onto transaction/recovery literature (Gray/Reuter, Bernstein/Hadzilacos/Goodman, Kleppmann, Helland, Lamport, Gray & Lamport); proposes PATH-052/H23. | research note only, no new FIND/PATH claimed |

## Gaps / inconsistencies

- **PATH numbers with no Evidence-line FIND document:** PATH-001–003, 005–007, 013–014, 018–026 never appear as the `Evidence:` line of any FIND doc. Some (013, 014) are mentioned only inline inside FIND-005's text; some (019, 026) are referenced inline in FIND-025 as "already documents elsewhere," but no corresponding FIND file exists in `docs/findings/` for any of these numbers.
- **Non-monotonic PATH-to-FIND assignment:** FIND-001 (earlier file) cites PATH-010/011, while FIND-002 (later file) cites the numerically earlier PATH-004/008/009.
- **Non-integer PATH id:** FIND-024's Evidence line is `PATH-046.5`, the only fractional PATH number in the set — breaks the otherwise-integer PATH sequence between PATH-046 (FIND-023) and PATH-047 (FIND-025).
- **FIND-003 has no `Evidence:` line** (only `Protocol:`), unlike every other FIND-0xx document.
- **"Canonical prior commit" only appears in FIND-027, FIND-028, FIND-029.** No earlier FIND document records a canonical commit, so commit provenance is not traceable for PATH-004 through PATH-047 from these docs alone.

## Current frontier

Latest canonical FIND is **FIND-029** (`docs/findings/FIND-029-...md`, Evidence: PATH-051, canonical prior commit PATH-050 at `da3afb5`). Latest research note is **PKV-TRANSACTION-RECOVERY-MAP-001** (`docs/research/`), which does not itself add a FIND or PATH result. That note explicitly proposes **PATH-052 (H23)** as the next PATH — concurrent recovery of a stuck, in-doubt `Reserved(K)`, testing whether FIND-027's existing claim-boundary lock, reused around the recovery decision, already prevents duplicate effect execution.
