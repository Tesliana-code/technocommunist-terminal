# FIND-026 — Durable Cross-Call Identity Memory Is Necessary for Replay Safety, and Degrades to `Unresolved`, Never to a Silent Guess, at Every Limit Tested

Status: OBSERVED / DERIVED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-048

## Correction notice

Cross-review found six issues across this finding's revisions, all
corrected below:

1. **A real resolver bug**: a `Reserved` (not yet `Settled`) record
   matched on occurrence key was returned as `Reserved` unconditionally,
   without comparing payload — so a conflicting delivery arriving before
   confirmation was misreported as agreement rather than `Conflicted`.
   Fixed in `path_048_reserve` and its naive sibling; regression-guarded
   by new case **PATH-048-K**.
2. **Overclaimed `ContractVersion` necessity.** The first revision said
   "trust-contract version must be historical: YES," but the actual
   drift-detection decision (PATH-048-G) is made by a separate field,
   `label_first_seen_namespace` — `ContractVersion` is stored per
   settlement record as metadata only and is never read back by any
   decision. Narrowed below to what is actually established: a
   *historical authority binding* must survive current-contract
   evolution. `ContractVersion`'s own necessity is NOT ESTABLISHED.
3. **Overclaimed pruning-horizon minimality.** `has_pruned` consults only
   MAP MEMBERSHIP in `pruned_horizon`, never the `CallId` value it
   stores. This experiment establishes the need for a durable "this
   namespace's history may be incomplete" fact, not that a numeric
   per-call horizon is itself minimally necessary structure.
4. **Minimality/sufficiency language stronger than proven.** "Minimum
   durable state," "smallest state machine," and "necessary and
   sufficient" are replaced below with "smallest surviving candidate
   tested" and "sufficient for the modeled cases tested," and the
   atomicity claim is narrowed to what sequential (non-concurrent) tests
   can actually show.
5. **AMNESIA != GENESIS — a remaining sufficiency boundary, found by a
   further cross-review pass.** `path_048_reserve`'s "namespace not in
   `ever_active_namespaces`" branch treated absence of remembered history
   as PROOF of true genesis, unconditionally returning `New`. It is not
   proof: an empty ledger is byte-identical whether a namespace genuinely
   never had activity before, or had activity this ledger's own durable
   state lost, was reset, or restored incompletely.
   Fixed by adding **`HistoryCompleteness`** (`AttestedComplete` / `Unknown`,
   per namespace, defaulting to the safe `Unknown`) as a sixth, EXOGENOUS
   piece of ledger state, never derived from `records` or
   `ever_active_namespaces` being empty — mirroring PATH-042/FIND-019's
   original exogenous-trust-axiom pattern one level up, applied to the
   LEDGER's own provenance rather than to any one event or source.
   Regression-guarded by eight new cases, **PATH-048-amnesia-A through
   G** plus a dedicated **AMNESIA theorem** test (see below).
6. **NEGATIVE LOOKUP REQUIRES COMPLETE HISTORY — the same bug, still live
   one branch over, found by a second further cross-review pass.** Item
   5's fix gated only a namespace's very FIRST sighting
   (`!ever_active_namespaces.contains(namespace)`). For an
   ALREADY-active namespace, a miss on a specific `event_id` skipped the
   completeness check entirely: if STABILITY held and nothing had been
   pruned, the resolver returned `New` regardless of completeness. This
   is unsafe under **PARTIAL RESTORE WITHIN THE SAME NAMESPACE** — a
   restore that recovers SOME of a namespace's real settlements but
   silently omits others, with the namespace still correctly marked
   active. Reproduced directly (see OBSERVED) before being fixed: the
   resolver returned `New` for an already-settled `evt-2` whose record
   simply hadn't been restored. Fixed by making the completeness check
   apply to EVERY negative lookup used to derive `New`, not just a
   namespace's first sighting — `HistoryCompleteness` is also renamed
   `AttestedComplete` (from `GenesisKnown`) to reflect that completeness
   can come from a verified migration/import and need not imply system
   genesis. Regression-guarded by four new required cases,
   **PATH-048-completeness-A through D**, plus a dedicated test proving
   `ever_active_namespaces` is derivable/redundant across every ledger
   state constructed by PATH-048 — not independently necessary state
   (see DERIVED).

## Hypothesis under test (H19)

Pairwise/in-batch identity semantics are insufficient for durable replay
protection. A replay-safe economic boundary requires some durable
cross-call identity memory whose retention and authority semantics are
explicitly defined.

## Background

FIND-025 (PATH-047) established source-scoped identity resolution within
one resolution call and left cross-batch replay explicitly unsolved,
stating: *"PATH-047 establishes pairwise/in-batch identity semantics; it
does NOT establish durable replay protection across separate resolution
calls."* PATH-048 takes that gap as its question: what durable state
prevents one real economic occurrence from being counted twice when its
deliveries arrive in different resolution calls? It reuses PATH-047's
type vocabulary unmodified (`SourceId`, `NamespaceId`,
`TrustedSourceContract`, `SourceIdContract`, `EventIdentity`,
`Path047CandidateEvent`) and adds a new layer: state that survives across
separate calls, which PATH-047's pairwise and single-batch functions
never modeled.

## Candidate tested

**`Path048Ledger`**, keyed on canonical **occurrence identity**
(`OccurrenceKey{namespace, event_id}` — `source_id` resolved through
AUTHORITY, not the raw delivery key), storing **settlement identity**
(`SettlementRecord::Reserved | Settled`, each carrying a payload snapshot
and `ContractVersion` metadata), plus four durable facts kept separate
from the detailed records: `ever_active_namespaces` (survives pruning —
"this namespace has settlement history at all"), `pruned_horizon`
(membership only consulted — "this namespace's history may be
incomplete," not the specific `CallId` value it stores),
`label_first_seen_namespace` (the historical authority *binding* — the
first namespace each raw label was ever observed resolving to), and
`HistoryCompleteness` (per namespace — `AttestedComplete` or `Unknown`; the
LEDGER's own attested provenance, never derived from any other field
being empty). Six distinct concepts — delivery identity, canonical
occurrence identity, settlement identity, trust-contract version,
temporal retention state, historical completeness — represented as six
distinct pieces of state, never collapsed into one "seen IDs" set; but
not every field within them is proven minimally necessary (see
Correction notice, items 2–3, and NOT ESTABLISHED).

Resolution (`path_048_reserve`) for one incoming delivery against this
ledger, as corrected by BOTH completeness passes:

```
resolve source_id -> namespace via the CURRENT contract; none -> Unresolved
if this label's namespace has drifted since it was first observed -> Unresolved
if an exact (namespace, event_id) record exists:                          -- POSITIVE lookup: completeness never consulted
    if the namespace lacks SEPARATION -> Unresolved
    if the record's stored payload matches this delivery's payload -> Reserved or AlreadySettled (per record state)
    else -> Conflicted   (applies to BOTH Reserved and Settled records — the first fixed bug)
else (no record):                                                          -- NEGATIVE lookup: completeness ALWAYS consulted, first or not
    if this namespace's HistoryCompleteness is NOT AttestedComplete -> Unresolved   (the second fixed bug: this now applies whether or not the namespace is already active)
    if this namespace has NEVER been active before -> New            (attested-complete, nothing else to be inconsistent with)
    else if the namespace lacks STABILITY, or retention has pruned this namespace -> Unresolved
    else -> New
```

The completeness check is now unconditional on any MISS — the earlier
revision applied it only inside the "never active before" branch, which
left an already-active namespace's miss on a *different* `event_id`
completely ungated (see Correction notice item 6).

## Adversarial cases, all passing (`cargo test --test pathological_cases path_048`, 28/28)

| Case | Attack | Result |
|---|---|---|
| A | no persistent state; retry lands in a separate call | double settlement succeeds — confirms H19's premise |
| B | remember only last-seen event_id per namespace; evt-1→evt-2→replay evt-1 | replay misclassified as new — bounded "last seen" is insufficient |
| C | raw historical key set; exact replay across calls, then naive (unmemoried) pruning | replay caught pre-prune; silently missed post-prune with no retained memory of the forgetting |
| D | canonical occurrence key vs. raw delivery key, same alias-replay scenario | raw key misses it; canonical key catches it — strictly stronger on this comparison |
| E | unstable-ID namespace, same occurrence as evt-1 then evt-2 | `Unresolved` — no key-only memory can resolve this |
| F | alias rotation, same namespace via two different raw labels | `AlreadySettled` — protection survives the rotation |
| G | trust-contract rebinds a label's namespace between calls | naive (current-contract-only) resolution silently double-settles; label-drift-guarded resolution (via `label_first_seen_namespace`) catches it as `Unresolved` |
| H | event reappears after the retention horizon prunes its record | `Unresolved`, not a false `New` — confirmed via a side-by-side unpruned/pruned contrast |
| I | same occurrence key reappears later with a changed payload, against a `Settled` record | `Conflicted`; original record verified unmodified |
| J | reserve without ever confirming (crash before settlement) | `Reserved` on retry — at-most-once safety holds for this sequence, but stuck (liveness, not the safety property) |
| J₂ | settlement applied without a prior durable reservation | at-most-once safety lost — retry settles as `New` |
| **K (new)** | conflicting payload against a `Reserved` (not yet `Settled`) record | `Conflicted` — regression guard for the fixed bug; original `Reserved` record verified unmodified |

Plus a dedicated **temporal identifiability theorem** test and an
end-to-end integration test feeding cross-call survivors into the actual
production `allocate_contribution_pool`.

**AMNESIA != GENESIS — the further cross-review correction, its own
adversarial table:**

| Case | Attack | Result |
|---|---|---|
| **Theorem** | true-genesis ledger vs. amnesiac ledger: `records` and `ever_active_namespaces` byte-identical (both empty) on both; identical current delivery K | attested genesis → `New`; unattested (lost history) → `Unresolved` — the strongest counterexample in this file |
| amnesia-A | clean first bootstrap, explicitly attested | `New` — the safe baseline |
| amnesia-B | process restart, durable ledger (including its attestation) preserved intact | `AlreadySettled` on replay — protection survives an honest restart |
| amnesia-C | process restart, ledger genuinely lost (nothing carried over, attestation included) | `Unresolved` on replay — never a false `New` |
| amnesia-D | partial restore: records + `ever_active_namespaces` restored, completeness attestation NOT | exact-key replay still caught (`AlreadySettled`); a namespace outside the restored set stays `Unresolved` — restoring data ≠ attesting completeness |
| amnesia-E | completeness marker FALSELY claims complete (real prior history was actually lost) | `New` — indistinguishable from a true attestation; axiom forgery, PATH-042/FIND-019's pattern recurring |
| amnesia-F | a namespace genuinely new after genesis vs. a different namespace explicitly marked Unknown, same ledger | `New` for the new namespace; `Unresolved` for the Unknown-overridden one — the two facts are independent and coexist on one ledger |
| amnesia-G | migration import: data imported without attestation, then the identical scenario with explicit attestation | `Unresolved` without attestation (importing data ≠ attesting completeness); `New` once explicitly attested |

**NEGATIVE LOOKUP REQUIRES COMPLETE HISTORY — the second correction's
own required cases:**

| Case | Attack | Result |
|---|---|---|
| completeness-A (REQUIRED, reproduces the bug) | ACTIVE namespace + partial restore: evt-1 and evt-2 both genuinely settled; only evt-1 restored; namespace correctly marked active; completeness Unknown; nothing pruned; AUTHORITY+STABILITY+SEPARATION all hold; incoming evt-2 with its ORIGINAL payload | `Unresolved` — confirmed to be `New` (wrong) before the fix via a scratch repro |
| completeness-B (REQUIRED) | Same shape as A, but completeness genuinely `AttestedComplete` and STABILITY established; evt-2 genuinely never existed | `New` — the fix gates on completeness, it does not forbid `New` on a miss altogether |
| completeness-C (REQUIRED) | EXACT HIT + Unknown completeness: retained evt-1, replay evt-1 | `AlreadySettled` — proves completeness gates NEGATIVE evidence only, never a positive record hit |
| completeness-D (REQUIRED) | EXACT HIT + Unknown completeness, changed payload | `Conflicted` — conflict detection also survives incomplete ambient history |
| ever_active_namespaces redundancy | Compare `ever_active_namespaces.contains(ns)` against `pruned_horizon.contains_key(ns) \|\| records.keys().any(...)` across never-active / active-unpruned / active-then-fully-pruned states | equal in all three — derivable/redundant across every ledger state constructed by PATH-048, not independent state |

`amnesia-D` was also amended in place (per instruction) to add a
same-namespace partial-restore assertion directly, since its original
form exercised only a *different*, never-touched namespace and so did
not exercise this bug at all.

## OBSERVED

1. **Temporal identifiability theorem, proven executably.** The same
   current delivery — identical namespace, event_id, and full payload —
   is fed against two independent ledger histories. One resolves
   `AlreadySettled`, the other `New`. Current-event data alone is
   identical in both; only past state differs, and the correct action
   differs with it. This is the strongest counterexample constructed:
   two histories, same current input, provably different correct
   actions.
2. **The Reserved-payload bug was real and is now regression-guarded.**
   Before the fix, PATH-048-K's scenario would have returned `Reserved`
   (implying agreement) for a delivery whose payload actually disagreed
   with the in-flight reservation. After the fix, both `Reserved` and
   `Settled` records compare payload identically before returning
   `Conflicted`.
3. PATH-048-D reproduces, at the cross-call layer, the exact structural
   comparison FIND-025 made at the single-call layer: a raw-key-only
   design misses an alias-mediated replay that a canonical-namespace-key
   design catches, on the identical input.
4. PATH-048-G's naive and guarded resolvers are run on the byte-identical
   contract-evolution scenario; only the guarded one — which retains
   `label_first_seen_namespace`, a per-label historical namespace
   *binding* — avoids a silent double settlement caused by contract
   rebinding alone, with no replay attempt involved at all.
   `ContractVersion` metadata is confirmed retained correctly (observable
   via the settlement record) but is not what the guard consults.
5. PATH-048-H's unpruned/pruned side-by-side shows the exact same replay
   attempt resolves `AlreadySettled` before pruning and `Unresolved`
   after — never a silent `New` in either case — using only membership in
   `pruned_horizon`, not its stored `CallId` value.
6. PATH-048-I and PATH-048-K together confirm both record states'
   original payload is byte-for-byte unmodified after a conflicting
   delivery is rejected — no overwrite, no averaging, no last-write-wins,
   regardless of whether the pre-existing record was `Settled` or still
   `Reserved`.
7. The integration test confirms that only genuinely new settlements
   reach `allocate_contribution_pool` across two separate calls, and the
   allocation is numerically exactly what it should be (alice's one real
   contribution, bob's one real contribution — 40/60 and 20/60 of the
   pool), not the double- or under-counted alternative.
8. **AMNESIA theorem, proven executably.** Two `Path048Ledger` values are
   constructed — one for a namespace that genuinely never had activity,
   one for a namespace whose prior settlement was fully lost — and their
   `records` and `ever_active_namespaces` fields are asserted equal
   (both empty). The identical current delivery is then fed to both:
   the attested one resolves `New`, the unattested one resolves
   `Unresolved`. This is a stronger counterexample than item 1 above,
   specifically because it holds on MORE of the pre-correction
   observable surface — not just settlement history, but the ledger's
   entire prior activity record — while still requiring different
   correct actions.
9. PATH-048-completeness-C/D confirm exact-key replay AND conflict
   detection are UNAFFECTED by missing completeness attestation —
   completeness gates every NEGATIVE occurrence-history lookup used to
   derive `New`, never a lookup that already finds a record. Restoring
   `records`/`ever_active_namespaces` without restoring the attestation
   is exactly as safe as restoring both, for any key the restore
   actually captured.
10. PATH-048-amnesia-E confirms a FALSE `AttestedComplete` attestation
    produces the byte-identical confident `New` a true one would — this
    candidate has no mechanism, and by construction could have none, to
    distinguish them from the ledger's own contents.
11. PATH-048-amnesia-F confirms `HistoryCompleteness` composes correctly
    per-namespace on one ledger: an attested-complete default and a
    namespace-level `Unknown` override coexist without one leaking into
    the other's verdict.
12. **The second bug was real and is now reproduced, fixed, and
    regression-guarded.** PATH-048-completeness-A: with the completeness
    check temporarily reverted (a scratch repro run during this
    correction, not committed), an already-settled `evt-2` whose record
    was omitted by a partial restore resolved `New` — a genuine
    double-settlement risk, with `ever_active_namespaces` containing the
    namespace (because `evt-1` HAD been restored) the only reason the
    first completeness fix's own gate was bypassed. After the fix, the
    identical input resolves `Unresolved`.
13. **`ever_active_namespaces` is derivable/redundant across every
    ledger state constructed by PATH-048, not independently necessary,
    after the second fix.** A direct per-ledger-state comparison test
    shows `ever_active_namespaces.contains(namespace)` equals
    `pruned_horizon.contains_key(namespace) ||
    records.keys().any(|k| k.namespace == namespace)` in every state this
    experiment constructs (never active; active with unpruned records;
    active then fully pruned) — no claim is made about ledger states this
    experiment did not construct. Once every miss unconditionally
    consults completeness, the field's remaining job — deciding whether
    STABILITY/pruning checks are even relevant — is recoverable from
    `records` and `pruned_horizon` alone, in the states tested.

## DERIVED

- **Necessity of SOME durable cross-call state is well supported; H19's
  "insufficiency" half was the easy part.** The PRIMARY necessity result
  is the temporal identifiability theorem: identical current-event input
  (identity + full payload) can require different correct actions solely
  because past state differs, therefore some historical observable is
  necessary — no function of current-event data alone can be correct
  across both constructed histories. The AMNESIA theorem strengthens this
  further, showing that historical COMPLETENESS/provenance, not merely
  settlement history, is itself part of that required observable.
  PATH-048-A is a narrower, OPERATIONAL witness alongside these
  theorems, not itself the necessity proof: it uses two independent
  `new_attested_genesis()` ledgers and shows that stateless resolution
  combined with optimistic/re-attested initialization at each call can
  double-settle — a concrete illustration of the theorems' consequence,
  not a separate general argument. The substantive result is what a
  candidate that PASSES these specific attacks requires: strictly more
  than "remember what you've seen" (B, C's naive form both fail), and
  strictly more than a raw key (D) — but this is sufficiency for the
  tested threat surface, not a proof that no smaller design could pass
  these same cases some other way.
- **The ledger's only failure mode, at every boundary tested, is refusing
  to answer — never answering wrong.** E (unstable ids), G (contract
  drift), H (expired retention), and now K (conflict against an in-flight
  reservation) are four unrelated causes of the same outcome:
  `Unresolved` or `Conflicted`, never a wrong `New` or a wrong
  `AlreadySettled`. This is achieved by the same discipline FIND-025
  already established at the single-call layer: every fact this
  candidate depends on (AUTHORITY, STABILITY, SEPARATION, retention
  completeness, and now label-binding stability) is either confirmed or
  the verdict degrades — but this is demonstrated only for the specific
  cases constructed, not proven as a general property of the design.
- **Retention is not a tuning parameter with a free lunch, though this
  experiment does not establish precisely how much retention state is
  minimally required.** PATH-048-H shows that once a namespace's history
  has been pruned even once, that namespace can never again return to
  confidently classifying a miss as `New` from this candidate's key
  history alone. Correction: an earlier revision attributed this
  degradation to `ever_active_namespaces`; per the redundancy result
  above, that field carries no independent information in the ledger
  states PATH-048 constructed. What actually represents "this
  namespace's history may be incomplete" in this candidate is retained
  truncation knowledge — `pruned_horizon` membership — together with the
  completeness discipline (a miss is gated on `HistoryCompleteness`
  regardless of pruning); `ever_active_namespaces` remains, in this
  candidate, only an O(1) convenience/cache, not the mechanism doing this
  work. What is established is the shape of the tradeoff (bounded
  retention vs. permanent full-confidence novelty-detection are in
  tension); what is NOT established is that the specific `pruned_horizon`
  structure tested (a per-namespace `CallId`) is the minimal way to
  retain that tradeoff's boundary fact — only membership was ever
  load-bearing in any test here.
- **Ordering and durability of the reservation step is the load-bearing
  requirement these sequential tests show; concurrency/atomicity is not
  tested.** J shows a reservation that never gets confirmed is safe
  (merely stuck — a liveness gap). J₂ shows the reverse failure — an
  economic effect applied without an antecedent durable reservation — is
  an at-most-once-safety failure. Both tests run single-threaded and
  sequentially; neither executes a race between concurrent reservations.
  "The reservation check-and-set must be atomic" is a reasonable DERIVED
  implication of the ordering result, not something this experiment
  demonstrates.
- **Contract evolution is a genuine, independent replay-safety hazard,
  not a corner case of AUTHORITY — but the mechanism that catches it is
  narrower than "contract versioning."** FIND-025 modeled AUTHORITY as a
  point-in-time fact (bound or not, in the current contract). PATH-048-G
  shows that even a permanently, correctly bound label can still break
  replay protection if its binding is allowed to *change*, unless the
  system separately retains what it resolved to before. What this
  experiment actually shows sufficient is retaining the *binding itself*
  (`label_first_seen_namespace`); it does not show that retaining a
  version *number* alongside settlements (`ContractVersion`) contributes
  to catching this or any other tested attack.
- **`ever_active_namespaces` alone was NOT sufficient — it conflates two
  facts that must be independent.** It records what ONE ledger instance
  has itself observed. It cannot, and structurally could never, record
  whether that instance's observations are the COMPLETE history since a
  namespace's true origin — those are different facts about different
  things (the instance's own experience vs. the namespace's real-world
  past), and an amnesiac ledger and a genesis ledger are identical on the
  first while differing arbitrarily on the second. `HistoryCompleteness`
  is not an enhancement to `ever_active_namespaces`; it is a genuinely
  separate axis this candidate was missing entirely before the first
  completeness correction.
- **ABSENCE != NOVELTY, at ANY grain — not just a namespace's first
  sighting.** The first completeness correction fixed only the case
  where a namespace's entire history was the missing thing. The second
  correction found the identical fallacy one branch over: a miss on a
  single `event_id` within an ALREADY-active namespace is just as much
  negative evidence, and requires the identical attestation, as a
  namespace's first sighting does — `ever_active_namespaces` containing
  the namespace was being treated as if it meant "and therefore this
  namespace's retained history is complete," which does not follow (a
  partial restore is the direct counterexample). Every "absence" this
  candidate has encountered across both correction passes — a missing
  record, an empty `ever_active_namespaces`, a namespace outside a
  partial restore's coverage, a specific `event_id` outside one — turned
  out to require an accompanying EXPLICIT fact (SEPARATION, STABILITY, a
  label-binding, or `HistoryCompleteness`) before it could license a
  confident verdict. The general principle, established now rather than
  merely the namespace-level special case: *a missing key means "not
  found in what was retained," never "never happened," unless the
  retained history being searched is itself attested complete over the
  relevant identity domain.*
- **`ever_active_namespaces` is derivable/redundant across every ledger
  state constructed by PATH-048, once the second correction is applied —
  it carries no independent information in those states.** Before the
  second fix, the field controlled a REAL behavioral fork (whether
  STABILITY/pruning were checked at all), so it carried decision-relevant
  information no other field alone captured. After the fix makes
  completeness checking unconditional on any miss, the field's only
  remaining job is answering "is there anything else in this namespace's
  retained history to potentially be inconsistent with," which is exactly
  `records.keys().any(namespace matches) ||
  pruned_horizon.contains_key(namespace)` — proven equal across every
  state this experiment constructs (never active; active, unpruned;
  active, fully pruned). This is not claimed as a universal invariant
  over every possible future ledger state, only over the ones tested. It
  survives in the struct as an O(1) convenience (avoiding a scan of
  `records` on every lookup); removing it and computing the union on
  demand would change performance, not behavior, and this candidate did
  not need to make that change to fix the bug.
- **Historical completeness is exogenous and unverifiable from inside
  this candidate — PATH-042/FIND-019's axiom-forgery result recurs
  exactly, one layer up.** PATH-042 showed a false trust axiom produces
  false-but-internally-consistent confidence about topology; PATH-048-
  amnesia-E shows the identical shape for a ledger's own provenance: a
  false `AttestedComplete` attestation is indistinguishable, from inside
  `path_048_reserve`, from a true one. This candidate can faithfully
  propagate an attestation; it cannot audit one. The rename from
  `GenesisKnown` to `AttestedComplete` keeps this framing explicit:
  completeness can come from a verified migration/import (PATH-048-
  amnesia-G) and need not imply system genesis, but in every case it
  remains something the resolver propagates, never verifies.

## NOT ESTABLISHED

- **Whether `ContractVersion` is necessary for any correct verdict.**
  Stored throughout as metadata, never read by `path_048_reserve`'s
  decision logic. No test here proves it materially changes a correct
  verdict; none was constructed to prove the opposite either. Left
  explicitly open rather than assumed either way.
- **Whether a numeric per-call retention horizon (`pruned_horizon`'s
  `CallId` value) is itself necessary structure**, as opposed to a
  simpler durable boolean/set fact ("this namespace has had truncation").
  Only membership was consulted by any decision tested; the stored value
  is one plausible real-world design (e.g., for a future finer-grained
  policy), not proven-minimal.
- **How real systems would supply retention capacity, or what an
  appropriate window size is.** `prune_namespace` is a manually-invoked
  test primitive; nothing here models a real retention *policy* (size-
  based, time-based, cost-based).
- **Whether the label-drift guard's own state
  (`label_first_seen_namespace`) needs its own retention policy.** It is
  currently unbounded (one entry per distinct raw label ever observed,
  forever) — smaller than full settlement history, but not proven bounded
  in any real deployment's label cardinality.
- **What should happen to `Conflicted`/`Unresolved` deliveries at the
  system boundary** — unchanged from FIND-025: this candidate establishes
  they must not be silently resolved one way or the other; what actually
  happens to them next (reject, page a human, quarantine) is still
  undecided.
- **Whether a real upstream occurrence identifier, or restored source
  STABILITY, is obtainable in practice** for namespaces like PATH-048-E's
  `flaky-webhook` — this candidate only proves no purely-downstream,
  key-only design can substitute for one; it does not evaluate how such
  an identifier would be sourced.
- **Whether the reservation check-and-set is atomic under concurrent
  access.** Not tested — no concurrency or race scenario is executed
  anywhere in PATH-048. Recorded as a DERIVED (reasoned), not
  demonstrated, requirement.
- **Multi-node/distributed durability** — this ledger is a single,
  in-process structure. Whether "durable" in a real deployment (a
  database write, a distributed log, a two-phase commit across services)
  changes any of J/J₂'s conclusions beyond the sequential ordering result
  already derived is not modeled.
- Whether this design should be implemented in production
  (`src/contribution.rs`, `src/distribution.rs`) — out of scope; test-local
  only, per this mission's constraints.
- **How a real deployment would source a trustworthy `HistoryCompleteness`
  attestation, or audit one after the fact.** This candidate models
  attestation as a caller-supplied fact (`new_attested_genesis`,
  `attest_namespace_history_complete`) and shows it is unverifiable from
  inside the resolver (PATH-048-amnesia-E) — it does not evaluate what
  real-world process (a deployment checklist, a signed migration
  manifest, an external audit) could make such an attestation
  trustworthy, nor whether that process itself could fail the same way.
- **Whether completeness needs finer granularity than per-namespace.**
  This candidate's `completeness` map is keyed by `NamespaceId` only. A
  namespace that is completeness-known for SOME event_id ranges (e.g.,
  partially migrated) but not others is not representable; whether this
  matters for any realistic migration shape is not evaluated.
- **Whether `HistoryCompleteness` itself needs retention/versioning** —
  it is modeled as a simple, unbounded, never-pruned fact per namespace
  here (like `label_first_seen_namespace`); whether a real deployment
  would need to revoke or expire an attestation is not evaluated.
- **Whether `ever_active_namespaces` should actually be REMOVED as a
  refactor**, now that it is shown redundant. Kept in the struct here
  (the derivation is proven equal, not applied as a code change) because
  removing it is a performance/implementation decision, not a
  correctness one, and was out of this correction's scope. Whether the
  redundancy proof generalizes to ledger states this experiment did not
  construct (e.g. a namespace pruned, then reactivated with new records)
  is not checked.

## Verdict

**H19 CONFIRMED**, with its two halves supported to different degrees:

- **Necessity** of durable cross-call state is strongly supported, with
  the TEMPORAL IDENTIFIABILITY THEOREM as the primary result: identical
  current-event input can require different correct actions solely
  because past state differs, so some historical observable is
  necessary — no function of current-event data alone can be correct
  across two constructed histories that differ only in past state. The
  AMNESIA theorem strengthens this further by showing historical
  COMPLETENESS/provenance is itself part of that required observable,
  not just settlement history. PATH-048-A is best read as an operational
  witness consistent with both theorems — it shows stateless resolution
  with optimistic/re-attested initialization at each call can
  double-settle — rather than as an independent proof of necessity in its
  own right.
- **Sufficiency** is established only for the modeled threat surface and
  only under the explicit AUTHORITY/STABILITY/SEPARATION assumptions
  inherited from FIND-025, **plus one further explicit qualifier, now
  stated at its full, correct scope: a HISTORICAL-COMPLETENESS
  assumption governing every negative occurrence-history lookup used to
  derive `New` — not only a namespace's first sighting.** `Path048Ledger`
  — corrected per the payload bug fix and BOTH completeness-correction
  passes above — is the smallest surviving candidate tested across
  PATH-048-A through K, amnesia-A through G, and completeness-A through
  D. It is not claimed to be globally or formally minimal — two of its
  fields (`ContractVersion`, and the `CallId` value inside
  `pruned_horizon`) are retained as plausible design elements whose
  necessity this experiment does not establish either way, one field
  (`ever_active_namespaces`) is now shown redundant rather than
  independently necessary (see DERIVED), and the `HistoryCompleteness`
  attestation itself is, by PATH-048-amnesia-E, an exogenous fact this
  candidate can propagate but never verify — sufficiency is conditional
  on that attestation being true, not merely present.

**Theorem (established across both completeness-correction passes):**
historical completeness is a property required to interpret NEGATIVE
lookup evidence. A missing key means "not found in what was retained,"
not "never happened," unless retained history is attested complete over
the relevant identity domain — this generalizes AMNESIA != GENESIS to
its correct scope: **absence != novelty**, whether the absence is an
entire namespace's missing history or a single `event_id`'s.

Four structural limits were found and are not solved, only correctly
surfaced: unstable event identity (PATH-048-E), retention expiry
(PATH-048-H), ledger initialization/provenance ambiguity (PATH-048-
amnesia theorem), and partial restoration within an already-active
namespace (PATH-048-completeness-A) each remove, within this candidate,
the ability to positively confirm novelty — for an event_id, for a
previously-active namespace, for a namespace's very first sighting, and
for a specific event_id within an otherwise-known namespace,
respectively — and the candidate responds to all four by returning
`Unresolved` rather than guessing in either direction.

No production code was modified. No payload-hash dedup and no
`contribution_id` heuristic were used anywhere in the candidate.
