# FIND-032 — Authoritative Fencing Epoch Must Be Durable AND Visible Before Effect Authority Transfers

Status: OBSERVED / DERIVED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-054

**Revision note:** narrowed after a HORUS cross-review. PATH-054-D2 was
added to test whether the candidate handoff protocol
(`path_054_takeover_and_publish`) revokes a stale former owner DURING
the persist-to-propagate window, not only for callers acting after it
completes. It duplicates. The original OBSERVED items A/B/C/E/F/G/H/I/
J/P/Q are unchanged; the claim originally built on D alone was narrower
than intended and is corrected throughout this revision. Durability
language is also corrected: `Path054AuthoritativeEpochStore` is an
in-memory, test-local, assumed-durable structure in the worlds that
assert its survival — this file never tests a real storage system.

## Hypothesis under test (H25)

For stale-owner fencing to be enforceable across recovery takeover, the
current fencing epoch must be both DURABLE/AUTHORITATIVE and VISIBLE TO
THE DOWNSTREAM EFFECT BOUNDARY before a newer ownership epoch is treated
as effect-authoritative. A fencing token carried only by the new owner
is insufficient if the downstream effect boundary has not yet learned
that a newer epoch exists.

```
PATH-053-F2: epoch ISSUED to B -> provider never told -> stale A accepted
PATH-054:    epoch ISSUED -> AUTHORITATIVE -> VISIBLE -> only then safe
```

## Five things kept separate throughout (never collapsed)

1. **EPOCH ISSUED** — the ownership system minted token N (PATH-054-A).
2. **EPOCH AUTHORITATIVE** — durably committed that N is current for K
   (PATH-054-B, E).
3. **EPOCH VISIBLE DOWNSTREAM** — the effect boundary can reject tokens
   < N (PATH-054-C, D).
4. **EFFECT EXECUTED** — the external effect actually happened.
5. **COMPLETION RECORDED** — completion evidence exists (PATH-054-N).

## Critical non-solution, stated first

Neither issuance (1) nor authority (2) alone fences anyone. Only
visibility (3), once actually established at the downstream boundary,
rejects a stale token — and even PERFECT epoch durability and
visibility together still cannot answer whether a prior, unrecorded
effect already happened (PATH-054-P) — FIND-029 is untouched by any of
this machinery.

## Test-local candidate

Four explicit steps, never fused except inside
`path_054_takeover_and_publish`, which is a plain SEQUENTIAL function
call (not a lock) making three steps happen in one order within one call
— see its own doc comment in `tests/pathological_cases.rs` for exactly
what it does and does not make atomic:

- `path_054_issue_epoch` — step 1, mirrors PATH-053's acquire logic
  (lease liveness + exclusion), but issues an epoch NUMBER only.
- `path_054_persist_authoritative_epoch` — step 2, writes into
  `Path054AuthoritativeEpochStore { current_epoch: HashMap<OccurrenceKey, u64> }`,
  the one recovery-visible, assumed-durable-in-the-model authoritative
  record per key (a test-local, in-memory structure — its survival
  across a "crash" is a modeling choice in the worlds that assert it,
  never an experimental proof of physical storage durability).
- `Path054FencedProvider::propagate_epoch` — step 3, writes into the
  provider's OWN `visible_epoch: HashMap<OccurrenceKey, u64>`, monotonic
  (never moves backward), with NO access back into the authority store.
- `Path054FencedProvider::execute_with_token` — step 4, accepts a token
  only if `token >= visible_epoch.get(key).unwrap_or(0)`; an unobserved
  key defaults to "accept anything," reproducing the PATH-053-F2
  baseline exactly.

No shared Mutex wraps these by default — every deterministic case (A–Q)
owns its structs directly and calls only the steps it chooses, stopping
short exactly where it wants to construct a gap. Wrapping everything in
one lock would have smuggled back the very atomicity this PATH studies
the absence of; PATH-054-R (the one optional thread test) is explicit
about what its own Mutex covers instead.

## OBSERVED

All items below are executable, in `tests/pathological_cases.rs`
(`path_054_*`), test-local only — no production code was touched.

1. **Baseline: issued but not propagated (PATH-054-A).** A owns epoch 1,
   lease expires, B is issued epoch 2 (step 1 only). Stale A executes
   first: ACCEPTED — `provider.visible_epoch` has observed nothing.
   Reproduces PATH-053-F2 as this PATH's own baseline.
2. **Authoritative, not yet visible (PATH-054-B).** Epoch 2 is durably
   persisted (`path_054_authoritative_epoch(&authority, &key) ==
   Some(2)`), but `provider.visible_epoch` is untouched. Stale epoch 1 is
   STILL accepted — the provider never consults the authority store
   directly. Durability alone is insufficient.
3. **Propagated epoch rejects the stale owner (PATH-054-C).** Epoch 2 is
   both authoritative AND explicitly propagated (hand-assembled steps).
   Stale epoch 1: `Err(Path054FencingRejection { attempted: 1, visible:
   2 })`. Epoch 2: accepted. `real_effect_count == 1`.
4. **Delayed exposure of `Granted` (PATH-054-D).** Using
   `path_054_takeover_and_publish` end-to-end: by the time the caller
   observes `Granted(2)`, propagation has ALREADY happened (asserted
   directly against `provider.visible_epoch`). Stale A, resuming strictly
   AFTER that call returns, is rejected. The mechanism is SYNCHRONOUS
   PUBLICATION inside one sequential call — not acknowledgment, not a
   separately-polled shared store. **Narrowed scope:** this shows safety
   only for a caller who acts strictly after the protocol call returns —
   see D2.
5. **The handoff-window witness (PATH-054-D2, HORUS cross-review).** The
   SAME protocol, stopped midway by hand: epoch 2 is issued and
   persisted as authoritative, but `propagate_epoch` is deliberately NOT
   yet called — `authority.current_epoch[K] == 2` while
   `provider.visible_epoch[K]` is still absent (asserted directly).
   Stale A calls `execute_with_token` with its OLD epoch 1 **inside**
   this window: ACCEPTED, honestly exposed and not patched to hide it.
   Propagation then completes and B's own epoch 2 also executes.
   `real_effect_count == 2` — **both executed.** "Publication-before-
   `Granted`" is NOT "atomic/revoking handoff": a stale former owner who
   never observes `Granted` at all, and so never waits for anything, can
   still execute during the persist-to-propagate window.
6. **Deterministic replay repairs a persist-before-propagate crash
   (PATH-054-E).** Epoch 2 authoritative; provider "crashes" before
   propagation ever runs (modeled by simply never calling it). Stale
   epoch 1 is accepted pre-repair. Reading the authoritative epoch back
   and replaying it into `propagate_epoch` repairs visibility: stale
   epoch 1 is rejected only AFTER the replay.
7. **Enforcement state survives a crash after propagation (PATH-054-F).**
   Epoch 2 already visible, no effect has run yet; nothing spontaneously
   clears provider state in this in-memory model. Stale epoch 1 remains
   rejected; epoch 2 may still proceed after the "restart."
8. **Provider epoch-memory loss is recoverable from surviving authority
   — the central recovery test (PATH-054-G).** B genuinely executes once
   (`real_effect_count == 1`) under intact enforcement, and a
   contemporaneous stale attempt is correctly rejected. The provider's
   own `visible_epoch` entry for the key is then lost (a restart of just
   that cache): the SAME stale epoch 1 is now accepted again, duplicating
   B's effect (`real_effect_count == 2`). Because the authority store
   still remembers epoch 2, replaying it repairs rejection again — no
   further duplication.
9. **Authority loss alone does not break provider enforcement; double
   loss is the real, unsolved boundary (PATH-054-H).** After a full
   authoritative-and-visible takeover, the AUTHORITY record is removed
   while the provider's own cache is untouched: enforcement is UNAFFECTED
   — the provider never depended on authority's continued memory, only
   on having been told once. Only when the provider's OWN cache is ALSO
   then removed does a stale token get accepted again, with no surviving
   source of truth anywhere to repair from.
10. **Out-of-order propagation never downgrades (PATH-054-I).** Epoch 2 is
   propagated, then a delayed, older epoch 1 arrives: `visible_epoch`
   stays at 2. Epoch 1 still rejected; epoch 2 still accepted. Isolated
   at the provider boundary, no ownership involved.
11. **Skipped epoch, no contiguity assumption (PATH-054-J).** The
    visible epoch jumps straight to 3 (1 and 2 never individually
    observed). Both 1 and 2 are stale; only 3 is current.
12. **Different keys, independent domains (PATH-054-K).** K1's full
    takeover-and-publish does not touch K2's still-unclaimed state.
    Per-key issuance counters are independent (both start at 1, not a
    shared global counter — contrast PATH-053). One effect per key.
13. **Alias convergence (PATH-054-L).** Two raw `SourceId`s resolving to
    the same canonical `OccurrenceKey` converge on the same
    ownership/authority/visibility domain: a second acquisition attempt
    through the OTHER raw label, before the first lease expires, is
    denied (`AlreadyRecovering`). Deterministic — alias resolution is not
    itself a race.
14. **Payload conflict (PATH-054-M).** Rejected before any epoch
    machinery is touched: `Conflicted`, ownership map empty, zero
    executions.
15. **Completion present (PATH-054-N).** `AlreadyComplete`, no takeover,
    no new effect.
16. **Already settled (PATH-054-O).** `AlreadyComplete`, zero effects.
17. **FIND-029 P0/P1 under PERFECT epoch propagation (PATH-054-P).**
    World 0 (no historical effect) and World 1 (historical effect
    already happened, completion lost) are each taken over through the
    FULL candidate success protocol — authoritative AND visible, no gap
    anywhere (this world only asserts state AFTER the protocol call
    returns; it does not by itself speak to the D2 window). Both worlds
    receive the SAME epoch and the SAME takeover verdict, with identical
    authoritative-and-visible state confirmed directly. Execution still
    duplicates in World 1 (`real_effect_count`: 1 vs 2). Perfect epoch
    correctness answers "is this token stale," never "did this effect
    already happen."
18. **Same-token replay, one boundary test only (PATH-054-Q).** The
    current epoch (2), presented twice, is accepted twice —
    `real_effect_count == 2`. Kept to exactly one test, unchanged in
    verdict from PATH-053-O; not re-litigated, not solved here.
19. **Baseline gap under real threads (PATH-054-R, supplementary,
    optional).** A and B are issued their epochs deterministically,
    single-threaded, beforehand (nobody ever propagates). Two threads
    then race `execute_and_release` for stale epoch 1 and current epoch
    2 concurrently, synchronized by one barrier; each locked call is its
    own separate critical section, never fused across the two threads.
    Both are always accepted regardless of interleaving order (the
    outcome is scheduler-independent by construction, since neither call
    reads ownership/ledger state to decide whether to proceed):
    `real_effect_count == 2`.

## DERIVED

- **H25 is CONFIRMED for the tested candidate.** Issuing a newer
  ownership epoch (step 1) is insufficient by itself to revoke a stale
  owner (A). Durably persisting that epoch as authoritative (step 2) is
  ALSO insufficient alone — the provider consults only its own local
  `visible_epoch`, never the authority store directly (B). Stale-owner
  rejection becomes enforceable ONLY once the current epoch has been
  explicitly propagated to the downstream boundary (C).
- **Delaying exposure of `Granted` protects only callers who wait for
  it — it does NOT make the handoff itself atomic or revoking.** The
  candidate protocol (`path_054_takeover_and_publish`) achieves
  publication-before-`Granted` by SYNCHRONOUS PUBLICATION inside one
  sequential call (D). But PATH-054-D2 stops the same protocol midway
  and shows a stale former owner — who never observes `Granted` and so
  never waits for anything — executes successfully DURING the
  persist-to-propagate window (`real_effect_count == 2`). The correct,
  narrowed answer to this mission's central question: once synchronous
  publication *completes*, callers acting strictly after completion
  observe enforceable stale-token rejection (D) — that is NOT the same
  as "epoch allocation can be safely separated from epoch
  publication/enforcement" in general, since a stale owner requires no
  permission and can act inside the window regardless (D2, alongside
  A/B as the counterexamples when the guarantee is skipped entirely).
  What mechanism would make the handoff itself indivisible with respect
  to such a stale owner is an open question, not solved here (see Stop
  Conditions).
- **Durability and visibility, modeled as separate structures in this
  test-local candidate, fail independently and repair independently —
  precisely, not simply "losing either reopens execution."** E and F
  show provider enforcement state, once established, survives an
  in-memory "crash" and can be deterministically REPAIRED by replaying
  the authoritative epoch, in the worlds where the authority record is
  modeled as surviving. G reproduces PATH-053-P in this candidate and
  shows the same repair-by-replay works as long as the authority record
  survives — the central recovery result. H shows the converse: authority
  loss ALONE does not break already-propagated enforcement, because the
  provider's own state, once set, does not depend on authority's
  continued memory. The precise statement: loss of downstream visibility
  reopens stale-token acceptance UNLESS a surviving authoritative source
  can restore it; loss of the authoritative source removes that repair
  path, although already-established downstream enforcement may
  continue working on its own; only when BOTH are lost together is there
  no tested recovery source at all.
- **Only the "provider lags behind authority" direction is unsafe.**
  Because `execute_with_token` accepts any token `>= visible_epoch`, a
  provider that has fallen behind can still always accept the CURRENT
  legitimate owner's (higher) token — it only fails to reject STALE
  (lower) ones. Under-propagation is a safety risk (stale duplication),
  never a liveness risk for the current owner, in this candidate.
- **Monotonicity and contiguity are independent properties, both
  isolated cleanly.** I shows `propagate_epoch` never regresses on a
  delayed, older delivery. J shows nothing assumes epochs are
  contiguous.
- **FIND-029's P0/P1 remains open even under PERFECT epoch propagation**
  (P) — the strongest form of this claim tested across this series:
  perfect fencing infrastructure answers only "is this token stale,"
  never "did this effect already happen."
- **Same-token replay remains a separate, still-open gap** (Q), kept
  deliberately unexpanded per this mission's brief.

## KNOWN FROM LITERATURE (context only, per PKV-TRANSACTION-RECOVERY-MAP-001.md; nothing above is claimed as tested beyond PATH-054's own candidate, and no source is claimed to prove this file's result)

- Fencing tokens (Kleppmann, *Designing Data-Intensive Applications*,
  ch. 8) are the named mechanism this file instantiates. The literature,
  in this experiment's reading, does not separately name "authoritative
  epoch persistence" and "downstream epoch visibility" as two distinct
  sub-steps the way this mission's brief does — that split is this
  file's own test-local construction, built specifically to make
  PATH-053-F2's gap independently diagnosable. It is not attributed to
  any cited source as a named result.
- Leases (Kleppmann, ch. 8), used here only insofar as
  `Path054Owner.lease_until` carries forward PATH-053's ownership-liveness
  mechanism unchanged; PATH-054 does not re-examine lease semantics.

## NOT ESTABLISHED

- That synchronous publication (this candidate's chosen mechanism for
  step 3) is the ONLY or universally necessary way to guarantee
  propagation-before-`Granted` — acknowledgment-based and other
  mechanisms are named in this mission's brief but not built or measured
  here.
- That per-key epoch counters (used throughout this file, unlike
  PATH-053's single global counter) are required rather than merely
  convenient.
- Any resolution of PATH-054-H's double-loss stop condition or
  PATH-054-D2's handoff-atomicity stop condition.
- That `Path054AuthoritativeEpochStore` models any real durable storage
  system — it is an in-memory, test-local structure whose survival is an
  assumed modeling condition in the worlds that assert it, never an
  experimentally proven storage guarantee.
- Real external provider, authority-service, or distributed-lock-manager
  behavior — `Path054AuthoritativeEpochStore` and `Path054FencedProvider`
  remain this experiment's own test-only constructions.
- Any consensus-necessity claim. This candidate's authority store is a
  single, unreplicated struct; nothing here claims replication or
  consensus is required for a real system, and nothing here claims it
  is NOT required either — that question is explicitly out of scope
  (see Stop Conditions).
- Any novel theoretical result. As in every prior FIND in this series,
  Section 2 of PKV-TRANSACTION-RECOVERY-MAP-001.md's mappings are
  standard, decades-old concepts, not claimed as original.

## Candidate central theorem

In the tested recovery-fencing architecture, downstream visibility is
sufficient to reject subsequently arriving older tokens once publication
has completed. Merely delaying exposure of `Granted` until after
publication does NOT by itself prevent a stale former owner from acting
during the persist-to-propagate handoff window (PATH-054-D2): that owner
never observes `Granted` and so is never gated by it. Loss of downstream
visibility reopens stale-token acceptance unless a surviving
authoritative source can restore it; loss of the authoritative source
removes that repair path, though already-established enforcement may
continue on its own; losing both together leaves no tested recovery
source.

**Mandatory corollary 1:** authoritative epoch propagation prevents
stale older-token execution only; it is not operation idempotency
(PATH-054-Q).

**Mandatory corollary 2:** correct epoch state does not resolve
FIND-029's historical-effect ambiguity — demonstrated here under
PERFECT epoch durability and visibility in both compared worlds
(PATH-054-P), the strongest form of this claim in the series so far.

**Mandatory corollary 3:** downstream epoch state must be monotonic;
delayed older-epoch publication must not downgrade enforcement
(PATH-054-I).

Scope is the tested candidate only: no universal distributed-lock
theorem, no real-provider claim, no consensus-necessity claim.

## Stop conditions reached — new blockers

1. **Canonical epoch source becomes an exogenous trust root**
   (PATH-054-H): once BOTH the authority store's record and the
   provider's own visible-epoch cache are lost together for the same
   key, nothing durable remains to replay from, and no
   replication/consensus mechanism is built to survive that.
2. **Handoff atomicity / revocation boundary** (PATH-054-D2): delaying
   exposure of `Granted` protects only callers who wait for it; it does
   not revoke a stale former owner during the persist-to-propagate
   window, because that owner never asks permission in the first place.
   How a newly authoritative epoch could become downstream-enforceable
   WITHOUT a window in which the superseded owner can still execute is
   recorded as an open question. Candidate future mechanisms, named but
   NOT built: downstream consulting the authoritative epoch directly
   instead of a propagated cache; an atomic publication/authority
   boundary; a revocation handshake; resource-side epoch authority.

Both documented, neither solved here — deliberately only these two, per
this mission's instruction not to build more than one new layer at a
time (D2's finding was folded into this revision rather than opening a
third).

Related, explicitly NOT separately exercised here: whether authority and
provider can disagree PERMANENTLY (not just transiently, as E/F/G show
being repairable) — this candidate's synchronous-publication mechanism
never produces forged/lost acknowledgments because it uses no
acknowledgment protocol at all; an ownership counter resetting
independently of authority was not constructed.

**Recommended next PATH:** PATH-055, choosing ONE of: (a) the handoff-
atomicity/revocation boundary (stop condition 2 — arguably the most
direct continuation of this mission); (b) trust/authenticity of the
epoch authority itself (who may write to
`Path054AuthoritativeEpochStore`, and what if that write is forged); (c)
same-token idempotency, still open since PATH-053-O/PATH-054-Q; or (d)
reconciliation between a surviving authority and a provider whose cache
disagrees with it in a way stronger than this file's "replay always
repairs" story covers (e.g. two different non-empty values in permanent
disagreement). This mission's brief is explicit that only one of these
should be taken up next.

**FIND-029 still remains open**, now demonstrated under the strongest
tested condition in this series (perfect epoch durability and
visibility, identical in both compared worlds) — PATH-054-P.
