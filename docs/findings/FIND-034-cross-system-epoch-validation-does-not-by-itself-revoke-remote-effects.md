# FIND-034 — Cross-System Epoch Validation Does Not By Itself Revoke Remote Effects

Status: OBSERVED / DERIVED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-056
Canonical prior commit: PATH-055 / FIND-033 at `37ad4ba`

## Hypothesis under test (H27)

When epoch authority and effect commitment live in separate,
independently failing systems, a local validate-and-send protocol
cannot inherit PATH-055 candidate 2's in-process ordering guarantee
merely by carrying a fresh epoch token across the boundary. The remote
effect resource must itself participate in whatever ordering/fencing
rule is expected to reject stale work.

## Two-system model, never fused

All executable in `tests/pathological_cases.rs` (`path_056_*`),
test-local only — no production code was touched, and no actual
process/network boundary was built. "System A" and "System B" are two
MODELED, independently mutable state machines (distinct Rust structs
with no shared reference or shared lock between them), standing in for
what would be separate systems in a real deployment. No real network,
no assumed exactly-once delivery, no assumed-truthful acknowledgment.

- **System A — `Path056EpochAuthority`.** Owns `current_epoch[K]` only,
  identical in shape to PATH-054/055's authority store.
- **System B — three separate, never-fused test-local candidates:**
  - **Candidate 0 — `Path056RemoteResourceBlind`.** No epoch memory at
    all; commits whatever it is asked to commit.
  - **Candidate 1 — `Path056RemoteResourceFenced`.** Its own
    `visible_epoch[K]`, populated only by an explicit `install_epoch`
    call — "a local copy of an externally issued epoch."
  - **Candidate 2 — `Path056RemoteEffectAuthority`.** `current_epoch`
    and effect commitment live on the SAME struct;
    `remote_advance_epoch` and `remote_validate_and_commit_effect`
    (the latter performing check-then-commit in one function call) are
    its only two operations — "the remote resource is its own authority
    for its own effect commits."
- **Cross-system messaging (case Q only)** is modeled as explicit,
  test-controlled steps — construct a `Path056InstallMessage` value,
  then separately choose to deliver it (`path_056_deliver_install_message`)
  or not, and separately choose to check it landed
  (`path_056_check_remote_installed`) or not. A caller carries only plain
  values (`OccurrenceKey`, an epoch token, a payload) between systems.

## OBSERVED

1. **Local preflight, remote blind (PATH-056-A).** A validates token 1
   locally against System A while epoch 1 is current (a plain
   comparison, proving nothing to System B). System A advances to
   epoch 2. A's already-authorized, now-stale request reaches the BLIND
   remote resource and commits (`real_effect_count == 1`). B's own
   current request also commits. `real_effect_count == 2` —
   PATH-055-C's witness, reproduced across a system boundary.
2. **Remote fence, current epoch visible (PATH-056-B).** Once the
   remote resource's OWN `visible_epoch` is 2, stale token 1 is
   rejected (`Err(Path054FencingRejection { attempted: 1, visible: 2
   })`) and current token 2 is accepted.
3. **Mandatory PACOV — authority ahead, remote behind (PATH-056-C).**
   System A advances to epoch 2; System B's own `visible_epoch` still
   says 1. A stale token-1 request arrives BEFORE propagation: ACCEPTED
   (`real_effect_count == 1`). Propagation (`install_epoch(2)`) then
   completes; B's current token 2 is also accepted.
   `real_effect_count == 2`. Moving fencing to the remote resource
   changes WHERE enforcement occurs, not whether the remote has current
   authority.
4. **Remote effect-resource authority, effect wins (PATH-056-D).**
   Token 1's validate-and-commit, run entirely inside System B's own
   indivisible operation, linearizes before System B's own subsequent
   `remote_advance_epoch(2)`: commits (`real_effect_count == 1`),
   unaffected by the later advancement.
5. **Remote effect-resource authority, revocation wins (PATH-056-E).**
   System B's own epoch advances to 2 before token 1 arrives:
   `Err(Path054FencingRejection { attempted: 1, visible: 2 })`,
   `real_effect_count` stays 0.
6. **System A ahead of System B (PATH-056-F).** System A's own view
   says epoch 2 is current; System B's own enforcement state (candidate
   1) still says 1. System B still ACCEPTS token 1 — System A's belief
   does not, by itself, cause System B to reject anything.
7. **System B ahead of System A (PATH-056-G).** The reverse: System A's
   own view still says epoch 1; System B's own state already says 2.
   System B REJECTS token 1 regardless of System A's belief.
8. **Out-of-order delivery to remote (PATH-056-H).** Install(2) then a
   delayed install(1): `visible_epoch` stays at 2 — no downgrade.
9. **Skipped epoch (PATH-056-I).** Remote jumps straight to 3; tokens 1
   and 2 both stale, though neither was individually observed.
10. **Different keys (PATH-056-J).** K1's fencing at the remote resource
    does not affect K2's independent domain.
11. **Alias convergence (PATH-056-K).** Two raw `SourceId`s resolving to
    the same canonical `OccurrenceKey` share one remote revocation
    domain.
12. **Payload conflict (PATH-056-L).** Rejected by the existing,
    unchanged ledger-level check before either System A or any System B
    candidate is ever touched (`real_effect_count` stays 0).
13. **Completion present (PATH-056-M).** Completion evidence for the
    exact payload already exists at the remote effect authority; no new
    effect (`real_effect_count` unchanged at 1).
14. **Already settled (PATH-056-N).** Settled ledger state alone
    requires no System B consultation (`real_effect_count == 0`).
15. **Same-token replay, one boundary test only (PATH-056-O).** Under
    the freshness-only remote candidate, the current token, presented
    twice, is accepted twice (`real_effect_count == 2`). Remote fencing
    != idempotency; not solved here.
16. **FIND-029 P0/P1 under remote effect-resource authority
    (PATH-056-P).** World 0 (no historical effect) and World 1 (a prior
    effect committed at the remote before revocation, evidence lost)
    reach identical remote current-epoch state and identical (absent)
    completion state; `remote_validate_and_commit_effect` returns the
    SAME verdict in both, yet `real_effect_count` diverges: 1 vs 2.
17. **Q1 — install message lost/delayed.** System A commits epoch 2
    locally; the install message to System B is constructed but never
    delivered. A stale token-1 request reaches the remote, which still
    has no idea epoch 2 exists: ACCEPTED. Unsafe for stale rejection.
18. **Q2 — unobserved confirmation / coordinator knowledge ambiguity.**
    System B DOES install epoch 2; System A never observes or confirms
    that installation (the test simply never calls
    `path_056_check_remote_installed` to update A's own bookkeeping).
    System A's own `Path056TakeoverStatus` stays `Pending` — ambiguous
    for coordinator progress. This is CONSISTENT WITH a lost
    acknowledgment, but the test does not construct or lose a transport
    object, so it does not uniquely model, or prove, message loss
    specifically — a missed check or a coordinator restart before
    checking would produce the identical observed state. A direct
    contrast in the SAME test (calling `path_056_check_remote_installed`
    against the same, already-installed remote) shows the epoch state
    was learnable all along and would have produced `ConfirmedInstalled`
    had System A observed it; meanwhile System B's own stale-rejection
    already works (`accept_and_commit(1)` returns `Err`), independent of
    what System A
    knows.
19. **Q3 — effect committed at the remote just before install.** Stale
    token 1 is accepted and committed while the remote's own visible
    epoch is still 1; only then is install(2) delivered.
    `real_effect_count` stays 1 after the install — the later install
    cannot undo the historical pre-install effect.
20. **Q4 — stale token arrives after install completes.** Install(2) is
    delivered first; the stale token-1 request arrives strictly
    afterward: `Err(Path054FencingRejection { attempted: 1, visible: 2
    })`. The safe case.
21. **Real threads confirm the coupling within System B alone
    (PATH-056-R, supplementary, optional).** One thread races
    `remote_validate_and_commit_effect(token=1)` against another racing
    `remote_advance_epoch(2)`, synchronized by one barrier, no sleeps,
    with the Mutex wrapping ONLY `Path056RemoteEffectAuthority` — no
    System A struct exists in this test at all. Both observed outcomes
    are among the two allowed orderings; the forbidden third (epoch
    already at 2, yet a stale commit still lands) never occurs by
    construction.

## DERIVED

- **H27 is CONFIRMED for the tested candidates.** A validate-and-send
  protocol does not inherit PATH-055 candidate 2's in-process ordering
  guarantee merely by carrying a fresh token across a system boundary
  (A): authorization proof at System A is not revocation enforcement at
  System B, and a blind remote resource duplicates exactly like
  PATH-055-C, now split across two systems instead of two function
  calls. Giving System B its own epoch memory (candidate 1, B) removes
  the blindness but reopens the exact propagation race PATH-054 already
  found, one layer later (C, Q1): if the remote's epoch state is a copy
  of something issued elsewhere, a lost or merely delayed install
  message leaves it exactly as exposed as no propagation at all. Only
  when the remote resource is made its own authority for its own commit
  (candidate 2, D/E) does the TOCTOU close for that resource's own
  state, structurally identical to PATH-055's in-process result — but
  this closes nothing about whether that resource's epoch state agrees
  with whatever ownership authority (System A) believes (F/G): the two
  systems can disagree, and neither is silently treated as globally
  correct — System A's belief governs ownership/coordination questions,
  System B's own state is the only thing that governs whether System
  B's effect commits.
- **The main adversarial case (Q1-Q4)**, built from explicit,
  test-controlled message-passing steps: Q1 shows a lost/delayed install
  message is unsafe for stale rejection — exactly as unsafe as no
  propagation attempt at all. Q2 shows that an UNOBSERVED CONFIRMATION —
  System A never checking, and therefore never learning, whether B
  installed the epoch — leaves coordinator progress ambiguous (System
  A's own bookkeeping never advances). This is consistent with a lost
  acknowledgment, but the test does not construct or lose a transport
  object, so nothing here uniquely proves message loss as the cause; a
  missed check or a coordinator restart before checking would look
  identical. The ambiguity is INDEPENDENT of whether System B already
  enforces correctly — B's own state does not need A's knowledge of it to
  be
  correct, but A cannot safely act on an assumption that B installed
  anything without that acknowledgment. Q3 shows a historical effect
  committed at the remote strictly before a later install cannot be
  undone by that install — the same "pre-revocation effects survive"
  property as PATH-055-D, now at the message-delivery-order boundary.
  Q4 shows the safe case: stale rejected once install has completed. Not
  overstated beyond what these four deterministic cases show — no
  general claim about acknowledgment protocols, retries, or message
  ordering beyond the single delivery/non-delivery step each case
  constructs.
- **H/I/J/K/L/M/N mirror PATH-054/PATH-055's** monotonicity,
  no-contiguity-assumption, per-key-independence, alias-convergence,
  payload-conflict, completion-present, and already-settled results at
  the remote-resource boundary — nothing about crossing a system
  boundary changes any of them.
- **Same-token replay remains separately open, not re-litigated (O)**:
  remote fencing answers "is this token stale," never "have I already
  seen this exact token."
- **FIND-029's P0/P1 remains open at the remote-resource boundary (P)**:
  a remote resource with perfect, indivisible revocation ordering still
  cannot distinguish "no prior effect" from "a prior effect already
  committed, evidence lost."

## Answers to this mission's primary questions

1. **What breaks:** the assumption that a LOCAL validation proof
   constrains a SEPARATE system's later decision; nothing forces System
   B to have seen, or to keep seeing, whatever System A currently
   believes.
2. **Is "authority validates, then client sends request" sufficient?**
   No (A, Q1) — it is exactly as safe as no fencing at the effect
   boundary unless the remote resource independently enforces
   something.
3. **Can the remote resource enforce current epoch itself?** Yes (B,
   D-G) — either candidate 1 (a locally-propagated copy) or candidate 2
   (its own authority).
4. **Does PATH-054's propagation race reappear one layer later if the
   remote learns epochs asynchronously?** Yes (C, Q1). Making the
   remote its own authority (candidate 2) sidesteps this only for that
   resource's OWN state, not for reconciling it with a separate
   ownership authority.
5. **What happens if epoch advancement and remote effect acceptance
   independently commit?** Without a shared ordering domain, the same
   TOCTOU as PATH-055-C reappears (A); sharing one ordering domain AT
   THE REMOTE RESOURCE ITSELF (never spanning both systems) removes it
   for that resource's own commits (D/E), confirmed under real
   concurrency in R using a lock that spans only System B.

## Authority ownership question (per tested candidate, not universal)

- **Candidate 0:** nobody governs the remote's commit at all; unsafe
  stale requests are always possible.
- **Candidate 1:** the remote holds a local copy of an externally
  issued epoch; it is only as current as its own most recent install.
- **Candidate 2:** the remote resource's own epoch state is
  authoritative for its own effect commits; System A (if present at
  all) governs ownership/intent only, never the remote's commit
  decision.

None of these three is declared universally correct; each has a stated,
tested consequence.

## KNOWN FROM LITERATURE (context only, per PKV-TRANSACTION-RECOVERY-MAP-001.md and the PKV protocol glossary; nothing above is claimed as tested beyond this PATH's own three System-B candidates, and no source is claimed to prove this file's result)

- Fencing tokens and stale-writer rejection (Kleppmann, ch. 8) remain
  the named mechanism candidates 1 and 2 instantiate as remote
  stale-token enforcement, now explicitly at a resource that may be a
  genuinely separate system. Candidate 0
  (`Path056RemoteResourceBlind`) is the deliberate NO-ENFORCEMENT
  control and instantiates no fencing mechanism at all.
- "Atomic commit" / "transaction commit" (Gray & Lamport, "Consensus on
  Transaction Commit") name a DIFFERENT problem this file does not
  build or claim to need: nothing here coordinates System A and System
  B into one atomic outcome, and the cases in this file are constructed
  precisely to show what happens WITHOUT such coordination.

## NOT ESTABLISHED

- Real payment/API/provider behavior — `Path056RemoteResourceBlind`,
  `Path056RemoteResourceFenced`, and `Path056RemoteEffectAuthority`
  remain test-local, in-process constructions standing in for "a
  separate system," never a real network peer.
- Distributed-transaction necessity, consensus necessity, or
  two-phase-commit necessity for reconciling System A and System B —
  this file never attempts such reconciliation and takes no position on
  whether it would require any of these.
- That candidate 2 (remote resource as its own epoch authority) is a
  universally correct architecture — only that it closes the TOCTOU for
  that resource's own state under the tested cases; candidates 0 and 1
  remain valid TEST-LOCAL ARCHITECTURE SHAPES, and the tested cases
  expose stale-acceptance witnesses for them. This file did not inspect
  any real system, and makes no claim about which architecture a real
  system resembles or how safe a real system built that way would be.
- Exactly-once delivery or exactly-once effect semantics — Q1-Q4 model
  AT-MOST-ONCE delivery attempts explicitly (a message is either
  delivered once or not delivered at all in each case), never retried
  delivery.
- Any resolution of FIND-029's P0/P1 ambiguity (P) or same-token
  idempotency (O).
- Any novel theoretical result — Section 2 of
  PKV-TRANSACTION-RECOVERY-MAP-001.md's mappings and Gray & Lamport's
  named atomic-commit problem remain standard, decades-old concepts.

## Candidate central result

In the tested two-system model, validating an epoch in one system does
not revoke stale effects in another system. Stale-owner rejection
becomes a property of the effect resource only when that resource
itself enforces an epoch ordering at effect commitment. If its epoch
state is derived from a separate authority, disagreement or delayed
propagation between the two systems reopens stale-token acceptance
(candidate 1). Moving the revocation boundary into the effect resource
(candidate 2) closes the validate-to-remote-commit TOCTOU for that
resource's own state, but does not by itself solve synchronization
between ownership authority and effect authority.

Scope carefully: tested candidates only. No real network, no
exactly-once-delivery claim, no consensus necessity, no two-phase-commit
necessity, no universal-architecture claim, no novelty claim.

## Stop conditions reached — new blockers, not solved here

1. **Remote effect authority and ownership authority can permanently
   disagree.** F/G construct only a transient mismatch (one system
   ahead of the other at a snapshot in time); nothing here tests
   whether the two can PERMANENTLY diverge with no tested repair path,
   the way PATH-054-H's double-loss case did for a single
   authority/provider pair.
2. **System A lacks a tested recovery protocol when confirmation of
   B's installed state is not observed (Q2).** This can include a lost
   acknowledgment, a missed check, a coordinator restart before
   checking, or other causes — the test does not distinguish which.
   System A's own `Path056TakeoverStatus` has no tested way to recover
   from this within this file (no retry, no re-query protocol was
   built) — it simply stays `Pending` forever in the case as
   constructed.

Neither is solved here, per this mission's instruction not to build the
next layer in this same PATH.

**Recommended next PATH:** PATH-057, choosing ONE of — reconciling
System A's ownership authority with System B's own effect-commit
authority when the two can permanently (not just transiently) disagree
(stop condition 1, arguably the most direct continuation of this
mission); a re-query/retry protocol for System A to recover when
confirmation of B's installed state is not observed, without assuming
message truth (stop condition 2);
same-token idempotency (PATH-053-O/PATH-054-Q/PATH-055-N/PATH-056-O,
still open); or FIND-029's historical-effect ambiguity (still open,
PATH-056-P). Only one of these should be taken up next.

**FIND-029 still remains open**, now demonstrated at the remote-resource
boundary (PATH-056-P), alongside PATH-054-P and PATH-055-O's earlier
demonstrations at the in-process authority and in-process effect-commit
boundaries respectively.
