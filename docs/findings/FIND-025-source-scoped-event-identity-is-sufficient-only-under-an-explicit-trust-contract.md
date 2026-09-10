# FIND-025 — Source-Scoped Event Identity Requires Three Independently-Verifiable Exogenous Facts, Not One Combined Trust Label

Status: OBSERVED / DERIVED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-047

## Correction notice

This finding has been revised twice under cross-review; both defects are
closed in the model described below.

**First defect (STABILITY/SEPARATION conflation).** An earlier revision
modeled H18's conditions 2 and 3 behind one combined flag and reported
H18 confirmed on that basis. Cross-review found this conflated two
logically independent properties: its "loss of stability" test actually
reused the same `event_id` twice and so exercised loss of condition 3,
not condition 2 — the model could not distinguish them. Fixed by
representing STABILITY and SEPARATION as two independent boolean fields.

**Second defect (AUTHORITY/namespace boundary).** The revision that fixed
the first defect still treated "two source_id strings differ" as
DEFINITIONAL grounds for `Distinct`, requiring no trust lookup. Cross-
review found this unjustified: two different self-declared strings prove
only lexical inequality, never that they denote two different real
source namespaces — exactly the FIND-016/FIND-018 relabeling pattern this
repository already documents elsewhere (PATH-019, PATH-026), recurring at
the source-authority boundary. Fixed by resolving every `source_id`
string through the contract to a canonical `NamespaceId` before any
comparison, so cross-source `Distinct` now requires AUTHORITY for both
sides AND an explicit, contract-established namespace difference — never
string inequality alone. The batch resolver had a matching defect
(grouping by raw key let ungrounded claims bypass resolution by not
colliding); both revisions' grouping fixes are subsumed into the
namespace-based grouping described below.

## Hypothesis under test (H18, original statement)

A source-scoped identity `(source_id, event_id)` is sufficient for
replay-safe identity ONLY when: (1) `source_id` denotes an exogenously
trusted authoritative namespace; (2) within that namespace, `event_id` is
STABLE for replay of the same real occurrence; (3) within that namespace,
`event_id` SEPARATES distinct real occurrences.

## Amendment to H18

H18 as originally stated treats conditions 1–3 as one combined
requirement, and leaves condition 1 (AUTHORITY) underspecified for the
case of comparing TWO different source_id strings. The corrected
candidate shows the claim splits into three independently-checkable
sub-claims:

- **H18a (same-`event_id` direction):** for one namespace, a matching
  `event_id` is sufficient to conclude same-occurrence-or-conflict, given
  AUTHORITY + SEPARATION alone. STABILITY does not participate.
- **H18b (different-`event_id` direction):** for one namespace, a
  differing `event_id` is sufficient to conclude distinct-occurrence,
  given AUTHORITY + STABILITY alone. SEPARATION does not participate.
- **H18c (cross-source direction):** two different `source_id` strings
  are sufficient grounds for `Distinct` ONLY when AUTHORITY is
  established for BOTH strings AND the contract explicitly assigns them
  to DIFFERENT canonical namespaces. Lexical inequality between two
  strings is never, by itself, evidence of namespace inequality —
  neither string carrying AUTHORITY, nor only one carrying it, nor both
  carrying it without an explicit distinctness fact, license `Distinct`.

A namespace missing one of STABILITY/SEPARATION is not "less trusted" —
it is fully usable on the direction it does license and correctly
`Unresolved` (never wrong) on the other. AUTHORITY plus both properties,
for a single namespace, plus an explicit distinctness fact between
namespaces where cross-source comparisons are needed (H18 as originally
written) is the condition for a contract to be *fully* resolving on
arbitrary pairs; H18a/H18b/H18c are the conditions for *partial*,
still-safe use of a contract that provides less.

## Background

FIND-024 (PATH-046.5) established that `ContributionEvent` has no
authoritative identity today (CASE 3, proven blocker) and proposed,
without testing, a minimal `EventIdentity{source_id, event_id}` shape.
PATH-047 builds a test-local model of that shape and tests it against
thirteen adversarial cases.

## Candidate tested

**`EventIdentity{source_id, event_id}` + `TrustedSourceContract`.** Three
exogenous facts are modeled explicitly and independently — never
derived, verified, or fabricated from the events themselves:

- **AUTHORITY** (H18 condition 1) — whether a raw, self-declared
  `SourceId` string is bound to a real namespace at all, and to WHICH
  one. Modeled as `TrustedSourceContract::namespace_of: SourceId ->
  NamespaceId`. `NamespaceId` is a distinct type from `SourceId`
  specifically so that two different strings can be bound to the SAME
  namespace (deliberate aliasing) without that being indistinguishable
  from two strings denoting two real, different namespaces. A string
  absent from `namespace_of`, or one that merely resembles a bound
  string, carries no AUTHORITY.
- **STABILITY** (H18 condition 2) — whether one real occurrence always
  produces the SAME `event_id` across retries/redeliveries from a given
  namespace. Licenses exactly one inference: DIFFERENT `event_id` ⇒
  DISTINCT occurrence.
- **SEPARATION** (H18 condition 3) — whether distinct real occurrences
  always produce DIFFERENT `event_id`s from a given namespace. Licenses
  the other inference: SAME `event_id` ⇒ SAME occurrence.

STABILITY and SEPARATION are two independent boolean fields on
`SourceIdContract { stable, separating }`, keyed by `NamespaceId` in
`TrustedSourceContract::properties` — a fact about the real system, not
about any one label for it, so every alias of one namespace shares
exactly one set of properties. Resolution is a four-state verdict —
`Replay`, `Distinct`, `Conflicted`, `Unresolved` — never a boolean:

```
resolve source_id(a) and source_id(b) to namespaces via the contract
if either side has no AUTHORITY                 -> Unresolved
else if the namespaces differ                   -> Distinct   (namespace difference is explicit in the contract, never inferred from string inequality)
else if event_id matches:
    if not separating                           -> Unresolved (SEPARATION missing)
    else if payload matches                      -> Replay
    else                                          -> Conflicted
else (event_id differs):
    if not stable                                -> Unresolved (STABILITY missing)
    else                                          -> Distinct
```

`path_047_resolve_batch` resolves every event's `source_id` to a
namespace first; events whose `source_id` has no AUTHORITY are kept
apart (grouped by raw label, for traceability, but never merged into
`survivors`); authorized events are grouped by resolved `NamespaceId`
(never by raw `source_id`), then processed in two ordered phases per
namespace: PHASE 1 clusters by `event_id` and applies SEPARATION within
each cluster; PHASE 2 applies STABILITY across the resulting
per-`event_id` representatives before anything is allowed into
`survivors`.

## Adversarial cases, all passing (`cargo test --test pathological_cases path_047`, 13/13)

| Case | Scenario | Contract used | Verdict |
|---|---|---|---|
| A | same trusted namespace, same event_id, same payload | separation-only (stable=false) | `Replay` |
| B | same trusted namespace, different event_id, same payload | stability-only (separating=false) | `Distinct` |
| C | two source_id strings, contract explicitly assigns two DIFFERENT namespaces | both guaranteed, two namespaces | `Distinct` |
| D | untrusted/uncontracted source, matching event_id | absent from contract | `Unresolved` |
| E | same trusted identity, changed payload | separation-only (stable=false) | `Conflicted` |
| F | two DIFFERENT untrusted source_id strings, same event_id | none (neither authorized) | `Unresolved` |
| G | SAME occurrence, delivery=evt-1, retry=evt-2, namespace lacks STABILITY | separation-only (stable=false) | `Unresolved` — pairwise and batch-resolver level |
| G′ | same event_id twice, namespace lacks SEPARATION | stability-only (separating=false) | `Unresolved` |
| H | uncontracted, alias-looking source_id vs. a genuinely trusted one | one namespace contracted, spoof absent | `Unresolved`/`Unresolved` |
| I | same contributor/source/value, different authoritative event ids | stability-only (separating=false) | `Distinct`, both survive; `allocate_contribution_pool` confirms 80/100 |
| J | direction-independence: one namespace, both contract shapes tested | separation-only AND stability-only | each direction resolves only under its own property |
| K | AUTHORITY present, neither property guaranteed | neither-guaranteed | `Unresolved` in both directions |
| L | two source_id strings explicitly bound to the SAME namespace (aliases) | one namespace, two aliases | matching event_id via either alias still `Replay`; differing event_id still gated by STABILITY |

Every case except C/F/G/G′/H/J/L (which are specifically about the
AUTHORITY/namespace or STABILITY/SEPARATION boundary itself) deliberately
sets the property *irrelevant* to its direction to `false`, to prove that
property plays no role in that case's outcome.

## OBSERVED

1. `path_047_verdict` never inspects `contribution_id`, `contributor_id`,
   or any payload field to determine identity equality — only resolved
   namespace identity, then exactly one of SEPARATION/STABILITY depending
   on which direction the comparison needs. No forbidden key was used
   anywhere in the candidate.
2. PATH-047-F and PATH-047-H, under the corrected model, both resolve to
   `Unresolved` where a prior revision resolved them to `Distinct` —
   confirmed as a direct, reproducible before/after difference on the
   identical input data, not a hypothetical one.
3. PATH-047-L shows two different raw `source_id` strings bound to one
   namespace are resolved identically to one literal source: a matching
   `event_id` across the two aliases still collapses via SEPARATION (3
   input events → 2 survivors, not 3), and a differing `event_id` is
   still gated by STABILITY exactly as it would be for a single label.
4. PATH-047-C shows two source_id strings DO resolve to `Distinct`, but
   only because the contract explicitly assigns them separate
   `NamespaceId` values — pinned directly by asserting
   `authority_for(..).0` differs between them, not inferred from the
   test merely using two different-looking strings.
5. PATH-047-J/K (unaffected by this revision's correction) continue to
   show STABILITY and SEPARATION license disjoint directions, and that
   AUTHORITY alone (without either property) is exactly as unresolving as
   no AUTHORITY at all.

## DERIVED

- **AUTHORITY is namespace identity, not string membership.** Modeling
  `SourceId` and `NamespaceId` as distinct types is what makes "two
  labels, one real source" (aliasing, PATH-047-L) and "two labels, two
  real sources" (PATH-047-C) both representable and distinguishable. A
  model that conflates a raw label with its namespace — as the prior
  revision did — cannot express the difference and defaults to trusting
  string inequality, which is unfounded.
- **Lexical inequality between two source_id strings is not evidence of
  anything.** PATH-047-F (two unauthorized strings) and PATH-047-H (one
  authorized, one unauthorized/spoof-shaped) both resolve to `Unresolved`
  specifically because AUTHORITY — now correctly understood as
  namespace-resolved trust — is missing for at least one side; the
  strings merely differing contributes nothing to either verdict.
- **The AUTHORITY/namespace defect and the STABILITY/SEPARATION defect
  are the same class of error at two different points in the model:**
  both times, a fact that must be EXOGENOUS (does this label denote a
  namespace at all; does this namespace guarantee stability; does it
  guarantee separation) was instead treated as DERIVABLE from the data
  already at hand (a label's own string shape; an event_id match/
  mismatch alone). PATH-042/FIND-019's original finding — identity truth
  requires at least one exogenous trust axiom — applies recursively to
  every fact this candidate depends on, not just the top-level one.
- **Grouping by resolved namespace, not by raw key, is required at every
  layer of the batch resolver.** The first correction required grouping
  by `source_id` instead of the full `EventIdentity`; this correction
  requires grouping by `NamespaceId` instead of `source_id` — the same
  shape of fix, recurring because the underlying cause (deriving
  structure from unverified labels) recurs.
- **Identity resolution is a precondition to economic normalization, now
  including a namespace-resolution step.** `allocate_contribution_pool`
  has no field to carry any of AUTHORITY resolution, SEPARATION, or
  STABILITY through its formula (per FIND-024); all of it — resolving
  labels to namespaces, then applying SEPARATION, then STABILITY — must
  complete, externally, before any event reaches it.

## NOT ESTABLISHED

- **Cross-batch replay is not addressed and remains the most significant
  open question this finding leaves.** PATH-047 establishes pairwise and
  in-batch identity semantics only: `path_047_resolve_batch` is a pure
  function of the events handed to it in ONE call. A namespace's claim
  that does not collide with any other claim from that namespace WITHIN
  THAT BATCH is accepted as a lone survivor even without a STABILITY
  guarantee, because nothing in that batch could be wrongly merged with
  or split from it. This is sound only within one resolution call. If an
  unstable namespace's retries of the same real occurrence are split
  across separate resolution calls (a plausible real deployment shape —
  e.g. a retry arriving after the first batch has already been resolved
  and discarded), each call could independently accept a lone survivor
  and the system as a whole would still double-count. **PATH-047
  establishes pairwise/in-batch identity semantics; it does NOT establish
  durable replay protection across separate resolution calls.** This is
  not solved here — it is reserved as a future research boundary (a
  stateful, cross-batch resolution process, rather than a pure per-batch
  function, is one direction such research would need to evaluate).
- Whether real-world systems can actually supply a reliable
  `SourceId -> NamespaceId` binding (i.e. whether AUTHORITY, refined this
  way, is obtainable in practice — a PKI-bound source identity, a
  provisioned API-key-to-tenant mapping, etc.) or whether the aliasing
  mechanism itself becomes a new spoofing surface if the binding process
  is weaker than the identity claims built on top of it — not evaluated;
  this candidate only shows what the contract must express, not how a
  real system would populate it trustworthily.
- Whether `Conflicted`/`Unresolved` groups should be rejected outright,
  queued for manual review, or handled some other way at the system
  boundary — undecided.
- Whether `TrustedSourceContract` should be versioned, revocable, or
  auditable in a real implementation — this experiment's contract is a
  static, in-memory, per-call map only.
- Whether real-world namespaces partition cleanly into fixed
  `(stable, separating)` pairs, or whether either property could change
  over time — treated as fixed per test throughout.
- Whether this shape should be implemented in production
  (`src/contribution.rs`) — out of scope; test-local only.

## Verdict

**H18 CONFIRMED AS AMENDED (H18a + H18b + H18c).**

`(source_id, event_id)` is sufficient for replay-safe identity, per
direction, exactly as H18a/H18b/H18c state — never from the key alone,
and never from source_id string inequality alone, always jointly with
the specific exogenous fact (namespace-resolved AUTHORITY, plus SEPARATION
for same-event_id claims, STABILITY for different-event_id claims, or an
explicit namespace-distinctness fact for cross-source claims) that
licenses the comparison being made. Every case lacking the relevant fact
resolves to `Unresolved` or `Conflicted`, never to a guessed
`Replay`/`Distinct`; no heuristic dedup was introduced anywhere in the
candidate. This confirmation is scoped to pairwise and single-batch
resolution; it does not extend to durable replay protection across
separate resolution calls (see NOT ESTABLISHED). No production code was
modified.
