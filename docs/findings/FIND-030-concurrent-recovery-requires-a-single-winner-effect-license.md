# FIND-030 — Concurrent Recovery Requires a Single-Winner Effect License, and Does Not Resolve FIND-029's P0/P1 Ambiguity

Status: OBSERVED / DERIVED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-052

## Hypothesis under test (H23)

In the tested claim-before-effect recovery architecture: when the same
canonical occurrence is already durably `Reserved(K)`, completion
evidence is absent, and multiple recovery workers independently attempt
recovery, more than one worker can receive permission to execute the
same economic effect unless recovery itself has its own single-winner
boundary — distinct from PATH-049's `Absent -> Reserved(K)` claim
boundary, which does not by itself serialize a later recovery decision
over an occurrence it already claimed.

```
PATH-049:  Absent                        -> Reserved(K)
PATH-052:  Reserved(K) + completion unknown -> recovery-effect permission
```

## Critical non-solution, stated first

PATH-052 studies CONCURRENCY SAFETY (preventing two live recoverers from
duplicating each other), never TEMPORAL IDENTIFIABILITY (determining
whether an earlier, unrecorded effect already happened). FIND-029
established that P0 (effect never executed) and P1 (effect executed,
completion memory lost) are observationally identical on every
recovery-visible observable. PATH-052-H reconfirms this directly: the
tested single-winner candidate, run unmodified against P0 and P1, grants
exactly one recovery license in each world and still duplicates P1's
effect. **Serialization does not, and is not claimed here to, resolve
FIND-029.**

## OBSERVED

All items below are executable, in
`tests/pathological_cases.rs` (`path_052_*`), test-local only — no
production code was touched.

1. **PRIMARY LOSING WITNESS (PATH-052-A).** A naive recovery decision
   split into an explicit check step
   (`path_052_naive_check_recovery_permission`) and a separate act step
   (`path_052_naive_act_after_permission`), with nothing enforcing
   exclusion between them: two workers, forced deterministically (no
   threads) to both check before either acts, both observe `Granted`.
   `recovery_effect_licenses == 2`, `real_effect_count == 2`. Final
   completion-memory cardinality is explicitly NOT the safety metric:
   exactly one completion row and one `Settled` record survive despite
   two real executions (`provider.completion_memory.receipts.len() ==
   1`, `real_effect_count == 2`, in the same test).
2. **Reconfirmed under real threads (PATH-052-A2).** A naive check-then-
   act split under real concurrent threads, synchronized by TWO barriers
   (one before check, one forcing both checks to complete before either
   acts) reproduces `licenses == 2`, `real_effect_count == 2`
   deterministically. Without the second barrier this test is
   scheduler-dependent (observed directly: a fast worker can complete
   check-then-act before the other's check even runs, hiding the race) —
   recorded in that test's own doc comment; the deterministic PATH-052-A
   witness is authoritative regardless.
3. **Single-winner candidate (PATH-052-B/B2).** A single, indivisible
   critical section (`path_052_try_claim_recovery`, a `std::sync::Mutex`
   guarding `(Path052RecoveryClaims, Path048Ledger,
   Path051SplitProvider)` together) that checks local `Reserved`/
   `Settled` state, checks provider completion memory, checks an
   in-flight recovery-claim set, and inserts into that set — all while
   holding one lock — yields exactly one `Granted` verdict under
   concurrent racing; the loser observes `AlreadyRecovering`. Reproduced
   across 100 racing attempts (25 iterations x 4 workers,
   PATH-052-B2), zero flakiness observed. **This candidate's own logical
   linearization point is the `claims.in_flight.insert(key.clone())`
   call, executed while holding the Mutex — a property of THIS
   candidate, not a general theorem.** No claim is made that a Mutex, or
   serialization in general, is universally necessary; no claim is made
   that this is the only valid architecture.
4. **Different keys (PATH-052-C).** K1 and K2 each independently receive
   exactly one winner; no cross-key coalescing (`real_effect_count ==
   2`, one per key). **Recorded limitation:** the tested candidate locks
   the WHOLE `(claims, ledger, provider)` tuple per call, so EXECUTION of
   K1 and K2 is serialized relative to each other even though their
   ECONOMIC claim decisions are not coupled — the same reduced-
   parallelism property PATH-049-E recorded for its whole-ledger Mutex. A
   per-key lock was not built or measured here.
5. **Alias convergence (PATH-052-D).** Two raw `SourceId`s
   (`source-A`/`source-B`) resolving via AUTHORITY to the same canonical
   `NamespaceId`/`OccurrenceKey` — IDENTITY-LEVEL canonicalization,
   distinct from PKV-GLOSSARY-001's topology-level sense — converge on
   the same recovery claim domain: exactly one `Granted` across both
   raw labels. As with PATH-049-F, canonicalization
   (`contract.authority_for`) is resolved by the test caller BEFORE
   either thread touches shared state; this does NOT establish that
   canonicalization must physically execute inside the recovery lock,
   only that the synchronization domain must key on the resolved
   canonical identity.
6. **Payload conflict (PATH-052-E).** A recovery request against
   `Reserved(K, payload_A)` with `payload_B` is rejected as `Conflicted`
   before the provider is ever consulted: `real_effect_count == 0`, zero
   entries in the in-flight claim set.
7. **Completion already present (PATH-052-F).** Existing, matching
   provider completion memory reconstructs `Settled` locally and returns
   `AlreadyComplete` — mirrors `path_051_recover`'s own branch exactly;
   no `Granted` permission is issued, no new execution.
8. **Already settled (PATH-052-G).** `ledger[K] = Settled` yields
   `AlreadyComplete`, zero recovery licenses, zero executions.
9. **P0/P1 limit (PATH-052-H, the central scope guard).** World 0 (P0:
   effect never happened) and World 1 (P1: effect happened, completion
   memory lost) are constructed with IDENTICAL recovery-visible state
   (`ledger.records` equal, `query_completion` → `None` in both) —
   the same construction PATH-051-C used. The SAME single-winner
   candidate, run independently against each world, grants exactly one
   `Granted` license in EACH (zero races in either — concurrency safety
   holds) but produces `real_effect_count == 1` in World 0 (correct) and
   `real_effect_count == 2` in World 1 (duplicates the pre-existing
   effect). **Still unresolved**, exactly as FIND-029 already
   established for the un-serialized case.
10. **Recovery-owner crash (PATH-052-I, minimal probe, STOP condition
    reached).** Worker A is granted the recovery claim, then never calls
    the release step (models a crash before the effect). Worker B then
    observes `AlreadyRecovering` and can infer nothing further:
    `real_effect_count` stays 0, and B holds no license to attempt
    anything. The claim carries no owner identity, token, timestamp, or
    expiry in this candidate, so B cannot distinguish "A is still
    legitimately working" from "A crashed before the effect (safe to
    take over)." **Not solved here** — recorded as a new blocker per the
    mission's stop condition.
11. **Recovery-owner crash, dual-world (PATH-052-I2).** Directly executes
    the second world PATH-052-I only described in prose: World 0 (A
    crashes before the effect, as in PATH-052-I) and World 1 (A performs
    the real effect via `execute_real_effect`, then crashes before
    `persist_completion`/`confirm_settlement`/claim-release run) are
    constructed side by side. Every recovery-visible observable available
    to B — `ledger.records`, `provider.query_completion(&key)`, and
    `claims.in_flight.contains(&key)` — is asserted IDENTICAL between the
    two worlds before B acts. B then receives `AlreadyRecovering` in
    BOTH. `real_effect_count` (0 in World 0, 1 in World 1) is read only
    afterward, as a test oracle — never as recovery input. The "A crashed
    after the effect but before release" world is now an EXECUTED
    observation, not only an assertion in prose.
12. **False positive completion memory (PATH-052-J).** A forged
    completion record (inserted directly, bypassing
    `execute_real_effect`, identical construction to PATH-051-I) still
    yields `AlreadyComplete` and suppresses a needed effect
    (`real_effect_count == 0`). PATH-052's single-winner boundary does
    not cure FIND-029's soundness gap.
13. **False negative completion memory (PATH-052-K).** The effect
    genuinely happened once; completion memory records nothing. The
    single-winner candidate correctly grants exactly one license (only
    one worker ever runs — no race occurred) and duplicates anyway
    (`real_effect_count == 2`) — connects directly to PATH-052-H: the
    identical P0/P1 ambiguity, exercised via the false-negative
    construction instead of the direct dual-world construction.

## DERIVED

- **H23 is CONFIRMED for the tested candidate and tested cases, scoped to
  the tested claim-before-effect recovery architecture.** In this
  architecture, once `Reserved(K)` exists, concurrent recovery attempts
  require their OWN single-winner boundary before recovery-effect
  permission is issued if at-most-one live recovery execution is to be
  guaranteed — because PATH-049's claim-insertion critical section has
  already closed by the time recovery begins (PATH-052-H shows the claim
  boundary alone does nothing for the recovery decision). This is a
  genuinely NEW mechanism, not a restatement of PATH-049's: PATH-049
  governs `Absent -> Reserved`; PATH-052 governs `Reserved + unknown
  completion -> execute permission`. No claim is made beyond this tested
  architecture.
- **Concurrency safety and temporal identifiability are different axes**
  (PATH-052-H, K). Adding single-winner recovery control leaves FIND-029
  fully intact: it prevents concurrent recoverers from duplicating EACH
  OTHER, never whether an earlier, unrecorded execution already
  happened.
- **Authenticity remains orthogonal** (PATH-052-J, mirrors PATH-051-I /
  PATH-050-K/K2): single-winner concurrency control operates strictly
  downstream of whether completion evidence is authentic, and cannot
  fix a forged record.
- **Reduced parallelism is a property of this candidate's chosen
  simplicity** (PATH-052-C), not a requirement of the theorem itself.

## KNOWN FROM LITERATURE (context only, per PKV-TRANSACTION-RECOVERY-MAP-001.md; nothing below is claimed as tested by PATH-052)

- Mutual exclusion / locking (Bernstein, Hadzilacos & Goodman) is the
  general classical tool for "two actors, one contested decision" —
  `path_052_try_claim_recovery`'s Mutex instantiates this narrowly for
  the recovery decision specifically, not for the original claim
  decision (that was PATH-049's instantiation of the same general tool).
- Leases and fencing tokens (Kleppmann, ch. 8) are literature-named
  candidate machinery for the gap PATH-052-I/I2 expose — but they are NOT
  the same mechanism and do not solve the same half of the problem:
  - A **lease** can provide a time-bounded ownership/takeover rule (it
    says WHEN another worker may treat the owner as gone). Lease expiry
    alone does not prove whether the previous owner already caused the
    external effect: a stale owner may still execute the effect after
    its lease expires unless something downstream actually rejects it.
  - A **fencing token** can protect against a stale owner acting after
    losing ownership, but only when the downstream effect/resource
    boundary itself validates and rejects older tokens — the provider in
    this file has no such check, so a token alone would not close the
    gap either.
  Neither mechanism is claimed to already solve FIND-029's P0/P1 temporal
  ambiguity by itself; both require an external enforcement point (the
  effect/resource boundary) that this file's `Path051SplitProvider` does
  not implement. Not built or tested here beyond the plain Mutex
  candidate actually exercised. This lease-vs-fencing distinction is
  likely important for PATH-053.

## NOT ESTABLISHED

- That a Mutex, or serialization in general, is the ONLY or universally
  necessary single-winner mechanism for the recovery decision — only
  that the tested candidate achieves the tested property under the
  tested cases.
- That a per-key (rather than whole-tuple) lock would avoid PATH-052-C's
  recorded cross-key execution serialization — likely, per PATH-049-E's
  identical caveat, but not built or measured.
- Any resolution of PATH-052-I's recovery-owner-crash ambiguity — leases,
  fencing tokens, ownership epochs, and heartbeat-based takeover are all
  literature-named candidates; none is implemented or tested in this
  file.
- Real external provider behavior. `Path051SplitProvider` remains this
  experiment's own test-only construction, unchanged from PATH-051.
- Any novel theoretical result. As in every prior FIND in this series,
  Section 2 of PKV-TRANSACTION-RECOVERY-MAP-001.md's mappings are
  standard, decades-old concepts, not claimed as original.

## Candidate central theorem

In the tested claim-before-effect recovery architecture, once an
occurrence is already `Reserved(K)`, concurrent recovery attempts
require a single-winner boundary before recovery-effect permission is
issued if at-most-one live recovery execution is to be guaranteed. The
original `Absent -> Reserved` claim boundary does not by itself
serialize later recovery-effect decisions over an occurrence it already
claimed — that is the actual strong result PATH-052-H establishes, and
it is not weakened by this scope correction.

**Mandatory corollary:** such serialization prevents concurrent
recoverers from duplicating EACH OTHER, but does not resolve whether a
pre-crash external effect already happened.

No stronger wording than the above is claimed, even where individual
tests might suggest it.

## Stop condition reached — new blocker

`recovery ownership acquired -> owner disappears/crashes -> another
worker cannot safely decide whether to take over` (PATH-052-I).
Documented, not solved. **Recommended next PATH: PATH-053 — abandoned
recovery ownership / fencing / ownership epochs / leases.**
