# FIND-031 — Lease Restores Recovery Liveness; Fencing Rejects Stale Owners Only After the New Epoch Reaches the Effect Boundary

Status: OBSERVED / DERIVED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-053

**Revision note:** this FIND was narrowed after a HORUS cross-review of
the initial PATH-053 evidence. The original title ("... Effect Safety
Requires Enforced Fencing") overstated what PATH-053-F alone had proven.
PATH-053-F2, added in this revision, exposes a gap the original evidence
did not test: see finding D below. Nothing in the original OBSERVED
items A/B/C/D/E/F/G-N/O/P was wrong; the CLAIM built on top of F was too
broad.

## Hypothesis under test (H24)

A time-bounded or replaceable recovery-ownership mechanism can make
abandoned recovery claims reclaimable, but ownership expiry alone cannot
establish whether the previous owner already caused the external
effect. If a stale owner can still execute after losing ownership,
lease-based takeover alone does not guarantee at-most-one external
effect. A fencing mechanism can prevent stale-owner execution only if
the downstream effect boundary actually validates and rejects stale
fencing tokens.

```
PATH-052-I/I2: Recovering(K), owner gone -> claim never reclaimable, ambiguous
PATH-053:      Recovering(K), owner gone -> lease-expiry reclaim + fencing token
```

## Four axes, deliberately not collapsed

1. **OWNERSHIP LIVENESS** — can a stuck `Recovering(K)` eventually be
   reclaimed? (PATH-053-A/B)
2. **LIVE OWNERSHIP-CLAIM EXCLUSION** — can only one worker hold the
   current recovery ownership / receive the current ownership grant
   while the lease is live? (PATH-053-A, Q). This is a claim-grant
   property, not a claim about who can act at the downstream effect
   boundary — PATH-053-F2 shows those are different questions with
   different answers.
3. **EFFECT SAFETY** — can an old/stale owner still cause the external
   effect? (PATH-053-E/F/F2)
4. **TEMPORAL IDENTIFIABILITY** — did a previous owner already cause the
   effect before disappearing? (PATH-053-C/D/N; FIND-029, still open)

## Critical non-solution, stated first

Lease expiry answers axis 1 only. It does **not** answer axis 3 in
general (PATH-053-E/F2), and it cannot touch axis 4 at all (PATH-053-C/D,
N): even a *perfect* fix to axis 3 leaves FIND-029's P0/P1 ambiguity —
"did the effect already happen before I took over?" — untouched, because
that ambiguity is about the past, not about who is allowed to act now.

## Core findings (A–F)

- **A.** Lease expiry restores ownership liveness (PATH-053-A/B).
- **B.** Lease expiry does not solve FIND-029's historical P0/P1
  ambiguity (PATH-053-C/D, N).
- **C.** An effect boundary that has already observed a newer fencing
  token rejects an older one presented afterward (PATH-053-F, O).
- **D.** Advancing the ownership epoch alone does NOT immediately fence
  the old owner if the downstream authority has not yet learned the new
  epoch — a stale owner can win the race to the provider (PATH-053-F2).
  This is the gap the original FIND-031 draft did not test and did not
  claim to have solved, but whose absence made the title read broader
  than the evidence.
- **E.** Same-token replay is a separate, independently open gap: fencing
  token freshness is not operation idempotency (PATH-053-O).
- **F.** Fencing-epoch memory durability is another, separate boundary
  (PATH-053-P).

## OBSERVED

All items below are executable, in `tests/pathological_cases.rs`
(`path_053_*`), test-local only — no production code was touched. A
test-controlled logical tick (`now: u64`) drives lease expiry throughout;
no test sleeps.

1. **Abandoned claim before expiry (PATH-053-A).** A acquires ownership
   (`Granted`) and disappears. B attempts takeover one tick before the
   lease expires: `AlreadyRecovering`, zero B licenses, zero executions —
   LIVE OWNERSHIP-CLAIM EXCLUSION holds regardless of anything below.
2. **Lease expiry restores liveness (PATH-053-B).** At the exact tick
   the lease expires, B's takeover succeeds (`Granted`) with a token
   *strictly higher* than A's. Asserted as LIVENESS only — this test
   makes no claim about whether executing afterward is safe.
3. **Primary worlds / core proof for finding B (PATH-053-C/D).** WORLD A
   (A's lease expires, A never executed) and WORLD B (A's lease expires,
   A DID execute via the ground-truth-only path, no completion evidence)
   are constructed with every recovery-visible ownership observable
   available to B — `ledger.records`, `provider.query_completion`, the
   ownership entry's `token` and `lease_until` — asserted IDENTICAL
   before B acts (mirrors PATH-052-H's P0/P1 construction one boundary
   further out). B receives the **same** `Granted` takeover verdict
   (identical token) in both worlds. B then executes in both: World A
   yields `real_effect_count == 1` (correct — case C), World B yields
   `real_effect_count == 2` (duplicate — case D). **Lease expiry alone
   cannot tell B whether executing is safe.**
4. **Stale owner without fencing (PATH-053-E).** A's lease expires; B
   takes over and executes through the deliberately unfenced path
   (models a provider that does not enforce ownership tokens at all); A
   then resumes and executes too, through the same unfenced path.
   `real_effect_count == 2` — both executed.
5. **Fencing rejects the stale owner, AFTER the provider has seen the new
   epoch (PATH-053-F).** A owns token 1, A's lease expires, B takes over
   with token 2 **and executes first** (raising the provider's
   highest-accepted record to 2). A resumes and attempts to execute with
   its old token 1: rejected with an explicit
   `Path053FencingRejection { attempted: 1, highest_accepted: 2 }` **at
   the provider's own effect boundary** — not inside the ownership map.
   `real_effect_count == 1`.
6. **Stale owner executes BEFORE the provider observes the new epoch
   (PATH-053-F2, HORUS cross-review witness).** Same setup as F, but
   REORDERED: A owns token 1, A's lease expires, B takes over and is
   issued token 2 — but B has **not yet called the provider at all**.
   `provider.highest_accepted_token` has therefore never observed token
   2 (asserted directly). Stale A resumes and calls
   `execute_real_effect_fenced` with its OLD token 1 **first**: ACCEPTED
   — nothing has ever been recorded as higher for this key. B then
   executes with token 2: also accepted. `real_effect_count == 2`. **The
   tested highest-SEEN-token fence provides no immediate revocation the
   instant ownership changes hands; it only starts rejecting once a
   newer token has actually reached the provider.** This is the
   executed proof of core finding D.
7. **Current owner accepted (PATH-053-G).** The straightforward accept
   path: the caller holding the latest token executes successfully;
   ownership entry is released on completion.
8. **Different keys, independent domains (PATH-053-H).** K1's takeover
   after its lease expires does not affect K2's still-live ownership: a
   competing attempt on K2 at the same tick is still denied
   (`AlreadyRecovering`), while K1's is granted. Both keys execute
   exactly once, independently. Tokens are a single global monotonic
   counter in this candidate, but the accept/reject and lease-liveness
   decisions are keyed per-occurrence — no cross-key poisoning observed.
9. **Alias convergence (PATH-053-I).** Two raw `SourceId`s resolving,
   via AUTHORITY, to the same canonical `OccurrenceKey` converge on the
   same ownership/fencing domain under real concurrent threads (one
   barrier, no sleeps): exactly one `Granted` across both raw labels.
10. **Payload conflict (PATH-053-J).** Rejected before ownership is ever
    touched: `Conflicted`, `real_effect_count == 0`, ownership map empty.
11. **Completion already present (PATH-053-K).** Existing, matching
    completion evidence needs no takeover: `AlreadyComplete`, no new
    execution, no ownership entry created.
12. **Already settled (PATH-053-L).** `AlreadyComplete`, zero ownership
    entries, zero executions.
13. **False positive completion memory (PATH-053-M).** A forged
    completion record (bypassing `execute_real_effect_unfenced`) still
    yields `AlreadyComplete` and suppresses a needed effect
    (`real_effect_count == 0`). Lease/fencing machinery does not touch
    FIND-029's evidence-soundness gap.
14. **False negative completion memory (PATH-053-N).** The effect
    genuinely happened once (ground truth); completion memory records
    nothing. Only one worker ever runs — no live race, nothing for
    fencing to reject — and the single, honest `Granted` still
    duplicates (`real_effect_count == 2`). Connects directly to
    PATH-053-C/D: fencing stops stale *live* owners it has already seen,
    not unrecorded past ones.
15. **Token reuse / non-monotonic epoch, isolated at the provider
    boundary (PATH-053-O) — SAME-TOKEN REPLAY IS NOT IDEMPOTENCY.**
    After token 2 is accepted, replaying OLDER token 1 is rejected
    (`Path053FencingRejection`); but re-presenting the CURRENT highest
    token 2 again is still accepted (`>=`, not `>`, exactly the rule
    this mission specified), and executes AGAIN —
    `real_effect_count == 2` for two calls carrying the identical
    token. A fencing token can reject an older owner; it does not
    deduplicate repeated effect calls from the same valid, current
    owner. No exactly-once or at-most-once external effect semantics
    are claimed anywhere in this file.
16. **Ownership wrap / lost epoch memory — STOP CONDITION
    (PATH-053-P).** Token 1 is correctly rejected while the provider's
    `highest_accepted_token` record is intact. That record for the SAME
    key is then lost (models a restart/storage failure of *only* that
    memory — not ground truth, not the ledger). The identical stale
    token 1 is now silently **accepted**, duplicating the effect
    (`real_effect_count` goes from 1 to 2). **Not solved here** —
    recorded per this mission's stop condition.

## DERIVED

- **H24 is CONFIRMED for the tested candidate and tested cases, NARROWED
  after PATH-053-F2.** A lease-bearing, token-issuing ownership entry
  (`Path053RecoveryOwnership`) restores OWNERSHIP LIVENESS for an
  abandoned claim without weakening LIVE OWNERSHIP-CLAIM EXCLUSION while
  the lease is live (PATH-053-A/B, Q). PATH-053-C/D is the direct,
  executed proof that lease expiry answers liveness only: EFFECT SAFETY
  is a separate axis, unaddressed by lease expiry alone.
- **The tested fence closes the stale-owner execution subproblem only
  after the downstream boundary has observed a newer fencing epoch —
  not general effect safety.** PATH-053-E vs F is a matched pair showing
  fencing beats no-fencing when B executes before A resumes. PATH-053-F2
  is the necessary second matched pair, REORDERED: when A resumes before
  B's own execute call ever reaches the provider, A's stale token is
  accepted, because `highest_accepted_token` never learned about token 2
  in time. And PATH-053-O shows the fence does not solve the adjacent
  problem of same-current-token replay either: a call carrying the
  identical, still-current token duplicates the effect, because fencing
  freshness is not operation idempotency. The correct, narrowed
  statement of this mission's central distinction is therefore: **the
  tested highest-seen-token fence rejects stale OLDER-token execution
  only after the downstream boundary has observed a newer epoch** — not
  "fencing solves effect safety" unqualified, and not a claim that it
  deduplicates same-token replays.
- **Same-token replay is a distinct, independently open gap from stale-
  owner rejection** (PATH-053-O): fencing token freshness is not
  operation idempotency, and this candidate makes no idempotency claim.
- **Stale-owner prevention and historical-effect knowledge are
  different questions with different answers, exactly as instructed —
  and F2 adds a third, also-distinct question.** PATH-053-C/D/N
  reconfirm FIND-029's P0/P1 ambiguity is completely untouched by
  lease+fencing: an honest, uncontested `Granted` still duplicates a
  genuinely-past, unrecorded effect (N), and two recovery-visible-
  identical worlds still diverge in total effect count after an
  identical takeover verdict (C/D). PATH-053-F2 shows a THIRD question —
  "has the downstream boundary learned the current epoch yet?" — that is
  not the same as either of the first two, and would remain open even
  under a perfect fix to F2's gap.
- **Authenticity remains orthogonal** (PATH-053-M, mirrors
  PATH-052-J/PATH-051-I): lease/fencing machinery operates strictly
  downstream of whether completion evidence is authentic.
- **Fencing's own protection is contingent on durable epoch memory**
  (PATH-053-P) — a genuinely NEW dependency this candidate did not need
  to expose until fencing itself was added.

## KNOWN FROM LITERATURE (context only, per PKV-TRANSACTION-RECOVERY-MAP-001.md; nothing below is claimed as tested by PATH-053, and no source is claimed to prove this file's result)

- Leases (Kleppmann, *Designing Data-Intensive Applications*, ch. 8) are
  the literature's named mechanism for OWNERSHIP LIVENESS over an
  abandoned claim holder — `Path053Owner.lease_until`, checked against a
  test-controlled logical tick, instantiates this narrowly for the
  recovery-ownership decision.
- Fencing tokens (Kleppmann, ch. 8) are the literature's named mechanism
  for EFFECT SAFETY against a stale writer — but the literature is
  explicit, and PATH-053-E/F/F2 executes the distinction directly, that
  fencing tokens only protect anything when a downstream resource/effect
  boundary actually validates and rejects them, AND has already learned
  the current epoch by the time the stale caller arrives. Epoch
  propagation timing (the F2 gap) is not claimed as a Kleppmann result;
  it falls directly out of this candidate's own construction (a single
  shared `highest_accepted_token` map updated only on accepted effect
  calls, never on ownership acquisition).
  `Path053FencedProvider::execute_real_effect_fenced`'s `>=`
  highest-accepted-token check is this file's chosen enforcement point,
  not a claim that this is the only valid one.
- Stale writers / lock ownership are used here purely as descriptive
  terminology, matching PKV-GLOSSARY-001.md's usage; no claim that any
  cited source proves H24 for this candidate.

## NOT ESTABLISHED

- That a single global monotonic token counter, a `std::sync::Mutex`, or
  lease-based ownership in general are the ONLY or universally necessary
  mechanisms for reclaimable recovery ownership — only that this
  candidate achieves the tested properties under the tested cases.
- That per-key (rather than global) token counters would behave any
  differently — likely not, but not built or measured here.
- Any resolution of PATH-053-P's fencing-epoch-durability gap, or of
  PATH-053-F2's epoch-propagation gap — durable/authoritative epoch
  memory, propagating a newly-issued epoch to the downstream boundary
  BEFORE a takeover is considered enforceable, what happens if the
  provider cannot enforce fencing at all, and whether completion memory
  and fencing authority can disagree are all open, per this mission's
  stop conditions, not solved here.
- Any same-token-retry idempotency mechanism (PATH-053-O) — fencing and
  idempotency are shown to be different properties, neither solving the
  other.
- Real external provider, lease-service, or fencing-token-service
  behavior. `Path053FencedProvider` remains this experiment's own
  test-only construction, unrelated to any real distributed lock
  manager.
- Any novel theoretical result. As in every prior FIND in this series,
  Section 2 of PKV-TRANSACTION-RECOVERY-MAP-001.md's mappings are
  standard, decades-old concepts, not claimed as original.

## Candidate central theorem

In the tested abandoned-recovery model, lease expiry can restore
ownership liveness but cannot establish whether the prior owner already
caused the external effect. The tested highest-seen-token fence rejects
stale effects only after the downstream boundary has observed a newer
fencing epoch — it does not, by itself, immediately revoke a stale
owner's ability to execute the instant ownership changes hands, because
there is a gap between the ownership epoch advancing and the downstream
effect boundary learning that epoch (PATH-053-F2).

**Mandatory corollary 1:** enforced fencing, once its enforcement point
has observed the current epoch, prevents a *subsequently arriving*
stale-owner call; it does not resolve FIND-029's historical-effect
ambiguity for effects that may already have occurred before takeover.

**Mandatory corollary 2:** fencing token freshness is not operation
idempotency — this candidate makes no exactly-once or at-most-once
external effect claim, with or without fencing (PATH-053-O).

No stronger wording than the above is claimed, even where individual
tests might suggest it. Scope is the tested candidate only: no universal
lease/fencing theorem, no real-provider claim.

## Stop conditions reached — new blockers

1. **Fencing-epoch memory durability** (PATH-053-P): fencing-epoch
   memory itself is not attested durable/authoritative — losing it for
   one key silently reopens stale-owner duplication.
2. **Authoritative epoch propagation / downstream epoch visibility**
   (PATH-053-F2): advancing the ownership epoch does not by itself fence
   the old owner if downstream authority has not yet learned the new
   epoch; a stale owner can race epoch propagation and win.

Both documented, neither solved here — deliberately only these two, per
this mission's instruction not to build a cathedral. Related open
questions for PATH-054, not answered here: who owns the canonical
current epoch; when does downstream actually learn it; is epoch
advancement itself durably committed; can a stale owner race epoch
propagation (F2's direct demonstration that yes, in this candidate, it
can); what happens if downstream epoch memory is lost (P's direct
demonstration); can same-token retries duplicate independently (O's
direct demonstration, already answered: yes). **Recommended next PATH:
PATH-054 — authoritative/durable fencing epoch, plus its propagation to
the effect boundary, as one bounded next step; same-token-retry
idempotency is a related but separate question this mission explicitly
does not fold in.**

**FIND-029 still remains open**, and would remain open even under a
perfect fix to both stop conditions above: even perfect epoch
propagation does not answer whether A already executed *before* token 2
ever became authoritative (PATH-053-C/D, N).
