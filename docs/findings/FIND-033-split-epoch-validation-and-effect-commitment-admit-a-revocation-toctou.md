# FIND-033 — Split Epoch Validation and Effect Commitment Admit a Revocation TOCTOU

Status: OBSERVED / DERIVED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-055
Canonical prior commit: PATH-054 / FIND-032 at `d6dc292`

## Hypothesis under test (H26)

Closing PATH-054-D2's handoff window requires more than making the newest
epoch available to the effect boundary. If epoch validation and external
effect commitment can occur as two independently interleavable steps, a
stale owner may validate while its epoch is current, lose authority, and
still commit the effect afterward. Precise question: must epoch
advancement and effect authorization/commit participate in one ordering
domain for revocation to have a precise effect?

**Scope note:** "external effect commitment" is this hypothesis's own
framing, under test. What the two candidates below actually build and
run is **test-local modeled** effect commitment
(`real_effect_count` incremented in-process) — neither candidate
demonstrates a genuinely external provider (a different process,
service, or system) participating in the ordering domain. Whether a
real external boundary can is PATH-056's open question, not answered
here.

## Five moments kept separate throughout (PATH-055's own numbering)

1. **TOKEN / EPOCH ISSUED**
2. **EPOCH BECOMES AUTHORITATIVE**
3. **EFFECT BOUNDARY READS / VALIDATES EPOCH**
4. **EFFECT BECOMES IRREVERSIBLY COMMITTED**
5. **COMPLETION EVIDENCE IS PERSISTED**

PATH-055 concerns 2 <-> 3 <-> 4 only. Step 5 / FIND-029 is deliberately
NOT folded into any candidate's solution here.

## Two test-local candidates, never fused with each other

All executable in `tests/pathological_cases.rs` (`path_055_*`),
test-local only — no production code was touched.

- **Candidate 1 — direct authority consult**
  (`Path055DirectAuthorityProvider`). The effect boundary reads
  `Path054AuthoritativeEpochStore` (reused unchanged from PATH-054)
  directly at validation time; there is no propagated cache to lag
  behind. `path_055_validate_token_against_authority` and
  `path_055_commit_real_effect` are exposed as two SEPARATE,
  independently callable functions.
- **Candidate 2 — shared ordering domain**
  (`Path055AuthorityAndEffectState`). `advance_authoritative_epoch` and
  `validate_epoch_and_commit_effect` are both `&mut self` methods on the
  SAME struct; the latter performs check-then-commit inside ONE function
  call, with no externally observable half-state in between. The
  supplementary real-thread test additionally wraps the struct in one
  `std::sync::Mutex`. The concrete Mutex is a test-local implementation
  choice, not the theorem: what matters is that validate-then-commit is
  exposed as one function, never two independently callable steps.

## OBSERVED

1. **Baseline: reproduces PATH-054-D2 (PATH-055-A).** A owns epoch 1; B
   is issued and persisted as authoritative epoch 2; propagation is
   deliberately not yet called. Stale A executes inside the window;
   propagation then completes and B executes too. `real_effect_count ==
   2`. Anchors PATH-055 to PATH-054 exactly.
2. **Direct authority consult rejects a stale call arriving after
   revocation (PATH-055-B).** Epoch 2 is made authoritative BEFORE stale
   A calls. With no cache and no propagation step in this candidate at
   all, A's token 1 is rejected immediately:
   `Err(Path054FencingRejection { attempted: 1, visible: 2 })`. No
   propagation was needed because there is nothing to propagate to.
3. **THE CENTRAL ADVERSARIAL WITNESS — check/commit TOCTOU
   (PATH-055-C).** Authority epoch = 1. Stale-A-to-be validates token 1
   and SUCCEEDS (1 >= 1). Execution stops before commit. B's takeover
   makes epoch 2 authoritative. A resumes and commits from its earlier,
   now-outdated validation — `path_055_commit_real_effect` takes no
   authority reference and cannot detect the intervening revocation.
   B then validates its own current token 2 and commits.
   `real_effect_count == 2` — NOT patched away. Direct authority
   consultation alone does not close a check-to-effect gap.
4. **Atomic candidate, effect-wins order (PATH-055-D).** A's
   validate-and-commit (token 1, while epoch 1 is still authoritative)
   happens before epoch 2's advancement: A's effect commits
   (`real_effect_count == 1`), then epoch 2 advances, then B's own
   current token is separately accepted (`real_effect_count == 2`). A's
   effect is legitimate **relative to revocation ordering** — it
   linearized before revocation, and revocation cannot retroactively
   invalidate it — but the resulting two-effect **history** is not
   thereby proven economically correct; whether it is an intended
   double-payout or an unresolved duplicate is exactly what FIND-029
   leaves open (see PATH-055-O), not answered here.
5. **Atomic candidate, revocation-wins order (PATH-055-E).** Epoch 2's
   advancement happens BEFORE A's validate-and-commit: A's stale token 1
   is rejected (`Err`, `real_effect_count` stays 0); B's token 2 is then
   accepted (`real_effect_count == 1`).
6. **Current owner accepted after revocation (PATH-055-F).** Token 2 is
   accepted once epoch 2 is already authoritative, isolated from any
   stale-token interaction.
7. **Different keys, independent domains (PATH-055-G).** K1's revoked
   epoch 1 is rejected; K2, never touched by K1's revocation, accepts its
   own fresh token. This candidate's deterministic tests never wrap the
   struct in a lock; only the real-thread test (P) uses one Mutex over
   the whole struct — documented as a candidate limitation on
   cross-key parallelism, not a claim that per-key locking is necessary.
8. **Alias convergence (PATH-055-H).** Two raw `SourceId`s resolving to
   the same canonical `OccurrenceKey` converge on one revocation domain:
   revocation issued through one raw label is enforced against a
   validation attempt made through the other.
9. **Payload conflict (PATH-055-I).** Rejected by the existing,
   unchanged ledger-level check before either atomic candidate is ever
   consulted; `real_effect_count` stays 0.
10. **Completion present (PATH-055-J).** Once completion evidence for the
    exact payload exists, no new takeover or effect is needed; the
    candidate is correctly not re-invoked (`real_effect_count` unchanged
    at 1).
11. **Already settled (PATH-055-K).** Settled ledger state alone requires
    no effect; the candidate is never consulted (`real_effect_count ==
    0`).
12. **Out-of-order / old epoch (PATH-055-L).** Once epoch 2 is
    authoritative, old token 1 is rejected; a delayed, older
    `advance_authoritative_epoch(1)` arriving after epoch 2 does not
    downgrade it (monotonicity, same property as PATH-054-I at this
    candidate's own boundary).
13. **Skipped epoch (PATH-055-M).** The authoritative epoch jumps
    straight to 3; tokens 1 and 2 are both stale, though neither was ever
    individually observed; only 3 is current.
14. **Same-token replay, one boundary test only (PATH-055-N).** The
    current token (2), presented twice, is accepted twice —
    `real_effect_count == 2`. Revocation ordering has no per-call dedup
    mechanism. Kept to exactly one test, not re-litigated, not solved
    here.
15. **FIND-029 P0/P1 under the atomic candidate (PATH-055-O).** World 0
    (no historical effect) and World 1 (a prior effect legitimately
    committed before revocation, completion evidence lost) are both
    brought to the SAME current epoch and the SAME (absent) completion
    state via this candidate's own indivisible validate-and-commit path.
    `validate_epoch_and_commit_effect` returns the SAME verdict in both
    worlds, yet `real_effect_count` diverges: 1 vs 2.
16. **Real threads confirm the coupling under genuine concurrency
    (PATH-055-P, supplementary, optional).** A tries
    `validate_epoch_and_commit_effect(token=1)` while B tries
    `advance_authoritative_epoch(2)`, synchronized by one barrier, no
    sleeps. Both observed outcomes are among the two allowed orderings
    (A commits, then epoch settles at 2; or epoch reaches 2 first and A
    is rejected) — never the forbidden third (epoch already at 2, yet A
    commits from a stale pre-advance validation), which is structurally
    impossible here because validate-and-commit is one function running
    under one lock.

## DERIVED

- **H26 is CONFIRMED for the tested candidates.** Direct authority
  consultation (candidate 1) closes PATH-054-D2's specific "provider had
  not learned the new epoch" gap (B): reading authority directly removes
  the propagated cache that could lag behind it, so a stale call arriving
  strictly after revocation is rejected immediately with no propagation
  step required. But direct consultation alone does NOT close a
  check-to-effect TOCTOU gap: candidate 1 exposes validation and
  commitment as two separate, independently callable functions, and case
  C constructs a deterministic witness in which a stale owner validates
  successfully while current, an intervening revocation happens, and the
  same owner commits from its earlier, now-outdated validation —
  duplicating the effect. A fresh authority read proves only "token was
  current at the instant of the read"; it does not prove the token
  remained current until the effect irreversibly committed. Therefore:
  **fresh read != revocation atomicity**, and **authority visibility !=
  commit ordering**.
- **Candidate 2 closes the TOCTOU gap, for the tested cases, by making
  validate-and-commit one indivisible operation relative to epoch
  advancement.** Under either legal ordering — A's validate+commit
  linearizing before revocation (D), or revocation linearizing before A's
  validate+commit (E/F) — there is no execution equivalent to "A
  validates under epoch 1, epoch advances to 2, A commits as though epoch
  1 were still valid." An effect is forced onto one side of the
  revocation boundary: it either commits before the newer epoch becomes
  authoritative, or the older token is rejected afterward. Case D is
  explicitly not a duplicate-by-staleness: A's effect is legitimate
  relative to revocation ordering because it linearized before
  revocation, and revocation cannot retroactively invalidate it — but the
  resulting two-effect history is not thereby proven economically
  correct. Revocation ordering answers only which side of the boundary an
  effect fell on, never whether two effects for the same occurrence are
  an intended outcome or an unresolved duplicate. The real-thread test (P)
  shows the same coupling under genuine concurrency for this candidate's
  chosen single-Mutex construction: whichever operation acquires the lock
  runs to completion before the other can observe or change anything, so
  the forbidden interleaving never occurs by construction, not by
  scheduler luck.
- **G/H mirror PATH-054's per-key-independence and alias-convergence
  results at this candidate's own boundary,** with a documented
  limitation: the real-thread test's single Mutex reduces cross-key
  parallelism, which is a candidate limitation, not evidence that
  per-key locking is necessary.
- **I/J/K mirror PATH-054-M/N/O:** payload conflict, completion-present,
  and already-settled states are all handled before either atomic
  candidate is ever consulted, and closing the TOCTOU gap changes none
  of them.
- **L/M reconfirm monotonicity and no-contiguity-assumption** at this
  candidate's own boundary, exactly as PATH-054-I/J did at theirs.
- **Same-token replay remains a separate, still-open gap (N)**, kept
  deliberately unexpanded, unchanged in verdict from PATH-053-O /
  PATH-054-Q.
- **FIND-029's P0/P1 remains open even under this candidate's perfect,
  indivisible revocation ordering (O)** — the strongest form of this
  claim in the series so far at the validate-and-commit boundary: two
  worlds with identical current epoch and identical (absent) completion
  evidence, differing only in ground-truth history, receive the same
  verdict from `validate_epoch_and_commit_effect` and still diverge in
  total effect count.

## Mandatory corollaries

1. **Direct authority visibility closes propagation staleness but not a
   check-to-effect TOCTOU gap** (B vs. C).
2. **Revocation ordering does not provide operation idempotency** (N):
   the current token, presented twice, still executes twice.
3. **Revocation ordering does not resolve FIND-029's historical-effect
   ambiguity** (O): identical, perfectly-ordered epoch state in both
   compared worlds, divergent ground truth.
4. **A pre-revocation committed effect is not made invalid retroactively
   by a later epoch** (D): if A committed while epoch 1 was authoritative,
   epoch 2 cannot retroactively undo it. This is legitimacy relative to
   revocation ordering only — it does NOT establish that the resulting
   effect history is economically correct. PATH-055 defines the
   revocation boundary — on which side of revocation did the effect
   commit — not historical exactly-once.

## KNOWN FROM LITERATURE (context only, per PKV-TRANSACTION-RECOVERY-MAP-001.md; nothing above is claimed as tested beyond PATH-055's own two candidates, and no source is claimed to prove this file's result)

- Fencing tokens (Kleppmann, *Designing Data-Intensive Applications*, ch.
  8) remain the named mechanism both candidates instantiate at the
  validation step, unchanged from PATH-054's reading.
- Kleppmann ch. 9 names linearizability and idempotency/retries as
  related but separate concepts. This file does not claim to have built
  or tested general linearizability — only that one test-local
  candidate's validate-and-commit operation is indivisible relative to
  one other operation (epoch advancement) under the cases exercised here.
- "Check-then-act" / TOCTOU is used here as a plain descriptive term for
  the case-C gap; it is not attributed to any specific cited source as a
  named result in this experiment's reading of the literature map.

## NOT ESTABLISHED

- That `std::sync::Mutex`, or any single concrete locking/ordering
  mechanism, is the only or universally necessary way to make
  validate-and-commit indivisible — only that this candidate's chosen
  construction achieves it for the tested cases.
- That a REAL external effect boundary (a payment provider, a message
  queue, a third-party API) can participate in the same ordering domain
  as an epoch authority at all. `Path055AuthorityAndEffectState` is a
  test-local, in-process, single-struct construction. If a real external
  effect cannot share this ordering boundary, that is PATH-056's
  question, not answered here.
- That per-key locking is necessary rather than merely one option
  distinct from this candidate's single-Mutex real-thread test.
- Any resolution of PATH-054-H's double-loss stop condition, same-token
  idempotency (N), or FIND-029's P0/P1 ambiguity (O).
- Any consensus-necessity or database-transaction-necessity claim — this
  candidate's state is a single, unreplicated, in-process struct.
- Any novel theoretical result. General linearizability is
  Lamport/Herlihy-Wing-era literature this file does not claim to
  extend; Section 2 of PKV-TRANSACTION-RECOVERY-MAP-001.md's mappings
  remain standard, decades-old concepts.

## Candidate central theorem

In the tested recovery-fencing candidate, reading an authoritative epoch
before effect execution is insufficient if epoch validation and effect
commitment can be interleaved independently: a stale owner can validate
before revocation and commit after revocation (PATH-055-C). When epoch
advancement and validate-and-commit participate in one tested ordering
domain, an effect is forced to fall on one side of the revocation
boundary: it either commits before the newer epoch becomes authoritative,
or the older token is rejected afterward (PATH-055-D/E/F, confirmed under
real concurrency in P).

Scope carefully: tested candidates only. No universal Mutex theorem, no
universal linearizability theorem beyond the one candidate property
evidenced, no real-provider claim, no database-transaction necessity, no
consensus necessity, no novelty claim.

## Stop conditions reached — new blocker

**Whether a genuinely external effect boundary can share an ordering
domain with epoch authority at all is unresolved.** Candidate 2's
in-process, single-struct, single-Mutex construction is not available
once the epoch authority and the effect provider are different systems
(a different process, a third-party API, a separate database). This
candidate does not model, and does not claim to have solved,
cross-system atomicity between authority and effect. Per this mission's
stop-condition list, this is recorded as PATH-056's likely territory
rather than built here.

None of the other listed stop conditions were newly triggered: the
tested candidate never required the effect boundary to be a genuinely
different system (candidate 2 is one in-process struct, sidestepping
rather than resolving that question — hence it being recorded as the one
new blocker above); no independently failing internal boundary inside
effect commit itself was constructed; no validation acknowledgment
protocol exists in this candidate to be replayed after revocation; and
`Path054AuthoritativeEpochStore`'s trust/authenticity remains exactly as
untested as FIND-032 left it — named again here, not newly discovered.

**Recommended next PATH:** PATH-056, choosing whether/how a genuinely
external effect boundary (a real provider or resource in a different
process or system than the epoch authority) can be brought into one
ordering domain with epoch advancement, given that candidate 2's
in-process single-Mutex construction is not available once authority and
effect are different systems. Alternatives, named but not built here,
per this mission's instruction to take up only one: same-token
idempotency (PATH-053-O/PATH-054-Q/PATH-055-N, still open); FIND-029's
historical-effect ambiguity (still open, PATH-055-O); authenticity/trust
of the epoch authority writer itself (named in FIND-032's own
recommendation, still not taken up).

**FIND-029 still remains open**, now demonstrated under this candidate's
own indivisible validate-and-commit boundary (PATH-055-O), alongside
PATH-054-P's earlier demonstration under perfect epoch durability and
visibility.
