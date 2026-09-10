# FIND-028 — Reserved State Alone Cannot Safely Recover an External Economic Effect After an Independent Crash Boundary

Status: OBSERVED / DERIVED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-050
Canonical prior commit: PATH-049 at `3d38c74`

Result: safe recovery requires EITHER an authoritative completion
witness capable of distinguishing effect-happened from
effect-not-happened, OR an effect boundary whose retry semantics make
repeating the same canonical operation harmless for the tested failure
pattern — local `Reserved(K)` state alone is not enough. A bare
idempotency key/operation_id is never itself such a witness; it is
operation identity only.

## Correction notice

Cross-review found four precision issues, one substantive, all corrected
below:

1. **Receipt authenticity applies to BOTH surviving candidates, not only
   Candidate B.** The first revision's Verdict said PATH-050-K showed
   Candidate B's (queryable receipt) safety was conditional on receipt
   authenticity, implying Candidate A (provider-side idempotency) was
   not similarly exposed. Inspection of `EffectProvider::execute` shows
   its `IdempotentByOperationId` branch consults `self.receipts.get(&operation_id)`
   FIRST — the identical map `query_receipt` reads — so a forged receipt
   defeats Candidate A exactly as it defeats Candidate B. Confirmed by a
   new regression test, **PATH-050-K2**, and corrected throughout below.
2. **Crash matrix conflated ground truth with recoverable knowledge.**
   The original C1/C2 rows implied `Reserved(K)` itself made C1's
   completion status "knowable" ("yes — it did NOT happen"). Corrected:
   the matrix now separates GROUND TRUTH (what actually happened in a
   given world) from RECOVERABLE KNOWLEDGE (what the recovery candidate
   can infer from surviving local evidence) — C1 and C2 have opposite
   ground truth but IDENTICAL (unknown) recoverable knowledge.
3. **Idempotency key mislabeled as a completion witness.** A bare
   operation_id is operation IDENTITY, not evidence that completion
   occurred. What survives PATH-050-D/E/F/G/H is the PROVIDER's
   retry-idempotent semantics plus its own memory keyed by that
   identity — narrowed throughout below.
4. **"Byte-identical on every field" claim exceeded what PATH-050-C's
   executable assertion checked.** The original assertion compared only
   `.records`. `Path048Ledger` does not derive `PartialEq`. Corrected by
   (a) narrowing the claim to "identical on every observable the
   recovery candidates actually consult" (the theorem's true dependency
   — `.records` is the ONLY field either recovery function reads), and
   (b) adding explicit equality assertions for the remaining
   `Path048Ledger` fields as reinforcing (non-load-bearing) evidence.

## Hypothesis under test (H21)

An atomic occurrence claim is insufficient for exactly-once economic
completion when the economic effect and the durable `Settled`
transition do not share one failure domain. If the effect can succeed
while `Settled` remains unrecorded, a retry cannot distinguish "effect
happened, confirmation was lost" from "effect never happened" unless
some additional durable or externally verifiable completion evidence
exists. Candidate resolutions: an idempotency boundary at the effect
provider, durable effect receipt/evidence, transactional coupling, a
recoverable external operation identity, or another explicit completion
witness.

## Background

PATH-049 established that concurrent contenders for the same canonical
occurrence require a single-winner claim boundary (`Reserved(K)`) before
economic permission is issued, and explicitly left the atomic-economic-
effect boundary unsolved. PATH-050 attacks exactly that gap: given a
caller already legitimately owns `Reserved(K)`, what is the minimum
protocol to ensure the external economic effect is neither applied
twice nor silently lost, when effect execution, receipt persistence, and
local `Settled` recording can each fail independently?

## Candidate models

Three conceptually distinct candidates, per the mission brief:

- **Candidate C (local-only, losing)** — `Reserved`/`Settled` with no
  external witness at all. Two naive policies were tested:
  - `path_050_naive_blind_retry_recovery`: "if `Reserved(K)`: retry the
    effect, then write `Settled`." Loses to duplication (A).
  - `path_050_naive_never_retry_recovery`: "if `Reserved(K)`: never
    retry." Loses to permanent incompletion (B).
- **Candidate A (provider-side idempotency)** — `EffectProvider` in
  `IdempotentByOperationId` mode: `execute(operation_id, payload)`
  returns `Executed`, `SuppressedDuplicate{..}`, or
  `PayloadMismatch{..}`. `path_050_recover` calls this unconditionally
  once `Reserved` is confirmed to match the incoming payload locally.
- **Candidate B (queryable durable receipt)** — the SAME provider's
  `query_receipt(operation_id)`, consulted by `path_050_recover_via_
  receipt_query` BEFORE ever considering execution. Strictly safer with
  respect to duplication (it can never itself cause an execution) at the
  cost of being unable to originate an effect — only to recover a
  `Settled` state from evidence that already exists, or correctly stay
  `Unresolved`.

`EffectOperationId` is the canonical `OccurrenceKey` itself (via
`path_048_occurrence_key`) — never a raw `SourceId`, never a payload
hash, never `contribution_id`. This inherits, without re-deriving,
PATH-047/048/049's canonicalization guarantees: same occurrence → same
operation_id; distinct canonical occurrences → distinct operation_id
(tested: F, H); aliases → same operation_id after canonicalization
(tested: G). Global uniqueness beyond what `OccurrenceKey` already
provides is not separately claimed.

`EffectProvider` tracks two deliberately independent observables:
`execution_count` (real effect executions only, never incremented on a
suppressed duplicate) and `receipts` (a `HashMap<OccurrenceKey,
ContributionEvent>`). Nothing enforces these two fields agree — this is
what PATH-050-K and PATH-050-K2 exploit, against Candidate B and
Candidate A respectively, because both candidates read the SAME
`receipts` map: Candidate A's `execute()` checks it internally to decide
`Executed` vs. `SuppressedDuplicate`; Candidate B's `query_receipt()`
reads it directly. A bare operation_id is operation identity only —
never itself evidence that completion occurred; the evidence, such as it
is, is whatever `receipts` reports for that identity, and this candidate
has no way to verify that report.

## Crash-point matrix

GROUND TRUTH (what actually happened in a given world) is kept
explicitly separate from RECOVERABLE KNOWLEDGE (what the recovery
candidate can actually infer from `Reserved(K)` plus whatever local
evidence survives). Do not read "ground truth" as something
`Reserved(K)` itself makes identifiable — it does not; that is exactly
what C1 vs. C2 demonstrates.

| Point | Ground truth | Recoverable knowledge from surviving local evidence | This candidate's response |
|---|---|---|---|
| C0 — before `Reserved` | no claim exists yet | n/a | PATH-049's territory; out of scope here |
| C1 — after `Reserved`, before effect | effect did NOT happen | UNKNOWN — observably identical to C2 | blind retry HAPPENS to be correct in this world, but recovery cannot know it is in C1 rather than C2 (PATH-050-C) |
| C2 — after effect, before the *provider's own* receipt persists | effect DID happen; the receipt write did not persist | UNKNOWN — observably identical to C1 | blind retry is UNSAFE in this world; **not independently modeled as its own crash sub-step** — this fake provider's `execute()` performs effect + receipt as one atomic step, so C2 is reasoned about, not separately exercised (see NOT ESTABLISHED) |
| C3 — after a durable receipt exists, before `Settled` | effect happened AND a durable provider receipt exists | KNOWN, via query | recover from receipt, no re-execution (Candidate B, I) — CONDITIONAL on the receipt being AUTHENTIC (K, K2) |
| C4 — after `Settled` | `Settled(K)` locally recorded | KNOWN | `AlreadySettled`, unmodified from PATH-048 |

C1 and C2 are **identical in recoverable knowledge despite opposite
ground truth** — this, precisely, is the ambiguity PATH-050-C proves
executably. The theorem is not "C1 is knowable and C2 is not"; it is
that neither is distinguishable from the other using only what recovery
can actually observe.

## Adversarial cases, all passing (`cargo test --test pathological_cases path_050`, 12/12)

| Case | Attack | Result |
|---|---|---|
| A | effect succeeds, `Settled` write lost, naive blind retry | `effect_count == 2` — primary duplicate-effect witness |
| B | effect never happens, recovery refuses to blindly retry | `effect_count == 0`, `K` remains `Reserved` forever — safe, but permanently incomplete |
| C | WORLD A (effect happened) vs. WORLD B (effect never happened), identical `Reserved(K)` local state | naive retry: 2 vs. 1 executions from identical input; safe candidate: `Unresolved` in both — core identifiability result |
| D | idempotent provider, execute once, crash before `Settled`, retry same operation_id | provider suppresses duplicate, `effect_count == 1`, `Settled` written |
| E | same operation_id, changed payload on retry | `Conflicted`, no second effect, original receipt/record unchanged |
| F | two distinct event_ids, same namespace | distinct operation ids, both execute, `effect_count == 2` |
| G | two raw `SourceId`s aliased to one canonical namespace, same event_id/payload | same operation_id, `effect_count == 1` |
| H | two canonical namespaces, identical event_id string | distinct operation ids, both execute |
| I | receipt-queryable recovery, receipt exists (crash after effect, before local `Settled`) | recovers `Settled` from receipt, `effect_count` stays at 1, zero re-execution |
| J | no receipt exists, external state genuinely unknown | `Unresolved`, zero new executions, `K` remains `Reserved` |
| K | receipt inserted into provider memory without any real execution (forged), recovered via Candidate B (`path_050_recover_via_receipt_query`) | recovery reports `AlreadyComplete`; the needed real effect is silently suppressed, `effect_count` stays at 0 |
| K2 (cross-review addition) | the IDENTICAL forged receipt, recovered via Candidate A (`path_050_recover`, `execute()`-based) instead | `AlreadyComplete`, `effect_count` stays at 0 — CONFIRMS receipt authenticity is a trust boundary shared by BOTH surviving candidates, not unique to K's query-based path |

## OBSERVED

1. **PATH-050-C, proven executably.** `ledger_a.records == ledger_b.records`
   holds by direct assertion — the ONLY field either recovery candidate
   (`path_050_recover`, `path_050_recover_via_receipt_query`) ever reads.
   As reinforcing (non-load-bearing) evidence, the test also asserts
   equality of every other `Path048Ledger` field
   (`ever_active_namespaces`, `pruned_horizon`,
   `label_first_seen_namespace`, `completeness`, `default_completeness`)
   between the two worlds. The identical naive recovery procedure,
   applied to each world's independent ledger clone, produces
   `effect_count` 2 (World A: a genuine duplicate) vs. 1 (World B: a
   genuine, correct first execution) — `assert_ne!` on these two counts
   holds. Run again against each world's untouched, still-`Reserved`
   original state, the safe candidate (`path_050_recover` under
   `NonIdempotent`) returns `Unresolved` in BOTH worlds, touching neither
   world's provider.
2. PATH-050-A's naive blind-retry policy, when applied after a real
   prior execution plus a lost `Settled` write, produces `execution_count
   == 2` — a directly observed duplicate, not an inferred one.
3. PATH-050-D confirms `provider.execute()` under
   `IdempotentByOperationId`, called a second time for the same
   operation_id with the same payload, returns `SuppressedDuplicate` and
   leaves `execution_count` at 1; `path_050_recover` then writes
   `Settled` from that suppressed-duplicate signal.
4. PATH-050-E confirms the LOCAL ledger's own `Reserved` payload
   comparison catches a payload change before the provider is even
   consulted — `provider.execution_count` is asserted unchanged, and
   `provider.query_receipt` is asserted to still return the ORIGINAL
   payload.
5. PATH-050-G confirms `path_048_occurrence_key` resolves both
   `source-A` and `source-B` to the identical `OccurrenceKey` under the
   alias contract (`assert_eq!(key_a, key_b)`), and that delivering
   through each alias in turn yields exactly one `EffectExecutedNowSettled`
   and one `AlreadyComplete`, with `execution_count == 1` overall.
6. PATH-050-I confirms `path_050_recover_via_receipt_query` never calls
   `provider.execute()` — `execution_count` is asserted unchanged
   (staying at 1, the count from the ORIGINAL pre-crash execution) after
   recovery, while `ledger.records` transitions to `Settled`.
7. PATH-050-K confirms a receipt inserted directly into
   `provider.receipts` — bypassing `execute()` — is treated identically
   to a genuine one by `path_050_recover_via_receipt_query`:
   `AlreadyComplete` is returned and `execution_count` stays at 0,
   demonstrated in the same test as the exact same assertion pattern
   used for the genuine case (I), proving the candidate cannot tell them
   apart.
8. **PATH-050-K2 confirms the identical forged receipt also defeats
   Candidate A.** The SAME forged `provider.receipts` entry from K, now
   fed through `path_050_recover` (the `execute()`-based candidate)
   instead of the query-based one: `provider.execute()` finds the forged
   entry via its own internal `self.receipts.get(&operation_id)` check,
   returns `SuppressedDuplicate`, and `path_050_recover` confirms
   `Settled`. `execution_count` stays at 0 — the real effect is
   suppressed exactly as in K, proving the vulnerability is in the
   shared `receipts` memory itself, not in which recovery function reads
   it.

## DERIVED

- **H21 is confirmed for the constructed failure pattern.** PATH-049's
  atomic claim boundary answers "who may act," not "did the action
  durably happen." Once effect execution and `Settled` recording are
  allowed to fail independently (which this experiment assumes rather
  than derives — see NOT ESTABLISHED), `Reserved(K)` alone is
  insufficient for recovery, exactly as PATH-050-C demonstrates
  executably: identical current local state requires opposite correct
  actions purely as a function of un-observed past state.
- **The corollary candidate holds, narrowly, with precise terminology.**
  Safe recovery, for the failure pattern tested, requires EITHER an
  authoritative completion witness capable of distinguishing
  effect-happened from effect-not-happened (a receipt, actually
  consulted — I) OR an effect boundary whose retry semantics make
  repeating the same canonical operation harmless (provider-side
  idempotency — D). A bare idempotency key/operation_id is NOT itself
  such a witness — it is operation identity only; what survives D is the
  PROVIDER's retry-idempotent semantics plus its own memory keyed by
  that identity, which is the same memory a receipt-based recovery would
  read (see the next bullet). Neither candidate is claimed to be
  universally sufficient — only sufficient for the specific
  Settled-write-lost pattern this experiment constructs.
- **Provider-side idempotency and queryable receipts are the SAME
  underlying memory, exercised two different ways — not two independent
  solutions.** `IdempotentByOperationId`'s `execute()` and its
  `query_receipt()` read the same `receipts` map. The real design choice
  observed is WHEN to consult it: unconditionally on every retry
  (candidate A, D) versus only as a read before ever risking execution
  (candidate B, I/J). Candidate B is strictly safer against duplication
  (it structurally cannot cause a second execution) but strictly weaker
  at achieving completion on its own (it cannot originate the first
  execution; something else, eventually, has to call `execute()`).
- **A bare local `effect_status` bit written in a SEPARATE step after an
  independently-failing effect does not close the gap — it moves it one
  instruction later, structurally.** No separate test was built for
  this, because PATH-050-C already demonstrates why one is unnecessary:
  a bit written by the SAME local process that might crash between
  "effect happened" and "bit written" reproduces WORLD A exactly (effect
  done, confirmation lost) — the bit is just another local record with
  the identical failure mode as `Settled` itself. It is candidate C
  wearing a different field name, not a new candidate. This scope is
  deliberate and narrow: a bit TRANSACTIONALLY COUPLED to the actual
  effect (the effect and the bit write sharing one atomic failure
  domain, so neither can happen without the other) rests on a DIFFERENT
  failure-domain assumption than this experiment tests, and is not
  evaluated here either way — this finding does not claim every
  conceivable local bit is useless, only that one written as a separate,
  independently-crashable step is not a new candidate.
- **A receipt's usefulness is conditional on its durability boundary
  exceeding the local process's failure domain — a property this
  experiment assumes for its fake provider and does not independently
  verify.** The fake provider's `execute()` performs the real effect and
  the receipt write as one atomic, in-process step (there is no separate
  "receipt persistence" instruction to crash between). This is exactly
  crash point C2 in the matrix above, and it is NOT independently
  modeled — a real receipt store could itself fail to persist after a
  real effect executes, reproducing the SAME C1-vs-C2 ambiguity one
  layer further out. PATH-050 does not close this; it names it.
- **Receipt authenticity is a new exogenous trust boundary, SHARED by
  both surviving candidates — structurally identical to FIND-019's
  original finding and FIND-026's `HistoryCompleteness` attestation.**
  PATH-050-K/K2 are not contrived edge cases — they follow directly from
  the model's own construction (`execution_count` and `receipts` are
  independently writable with nothing enforcing agreement, and BOTH
  `execute()` and `query_receipt()` read `receipts`). Wherever a receipt
  is trusted without an independent authenticity mechanism, EITHER
  surviving candidate — not only the query-based one — can be made to
  skip a needed effect exactly as confidently as it recovers a genuine
  one. This was the strongest correction cross-review made to this
  finding: the vulnerability is in the shared completion-memory
  authority, not in a specific recovery code path.

## NOT ESTABLISHED

- **That effect execution and `Settled` recording CAN actually fail
  independently in any real system this repository might deploy
  against.** H21's premise is assumed as the experiment's starting
  condition (per the mission brief), not independently derived from any
  production code path — this repository has no real external effect
  provider to inspect.
- **Crash point C2 (receipt persistence itself failing, independent of
  the effect).** The fake provider's `execute()` is atomic by
  construction; whether a real receipt store's own durability gap
  reproduces the identical ambiguity was reasoned about (DERIVED, above)
  but not built or executed as a separate test.
- **Receipt authenticity / anti-forgery.** PATH-050-K/K2 demonstrate the
  vulnerability exists in this candidate's model, against BOTH surviving
  candidates, but do not evaluate any specific authenticity mechanism
  (signing, an independent audit trail, a trusted third party) — only
  that the absence of one is exploitable exactly as FIND-019/FIND-026
  already predicted at other layers.
- **Whether provider-side idempotency (candidate A) is achievable for a
  real payment/economic provider**, and under what operation_id
  stability/uniqueness guarantees from THAT provider — this candidate
  assumes the fake provider offers idempotency correctly; it does not
  evaluate any real provider's actual contract.
- **Whether a hybrid design (query-first, execute-with-idempotency-as-
  fallback) would be preferable to either candidate A or B alone.** Not
  constructed or compared; A and B were tested and compared only as
  presented in the mission brief.
- **Multi-occurrence batch recovery, ordering, or recovery-process
  concurrency itself.** PATH-049 already covers concurrent CLAIM
  contention; PATH-050 does not test concurrent RECOVERY attempts racing
  each other at the effect boundary (e.g., two recovery processes both
  calling `path_050_recover` for the same stuck `Reserved(K)`
  simultaneously) — this is a plausible follow-on question, not answered
  here.
- **Retention/expiry of provider receipts**, `ContractVersion`,
  `pruned_horizon`'s `CallId` value, and `ever_active_namespaces`'s
  redundancy — all unchanged and NOT newly established by PATH-050; no
  case here exercises pruning, contract evolution, or namespace
  first-sighting at all.
- Whether any of this design should be implemented in production
  (`src/contribution.rs`, `src/distribution.rs`) or integrated with a
  real database, HTTP effect provider, or queue — explicitly out of
  scope per this mission's constraints.

## Verdict

**H21 CONFIRMED for the constructed failure pattern.**

`Reserved(K)` alone — local claim state with no additional completion
witness — cannot safely recover an external economic effect once effect
execution and the durable `Settled` transition are allowed to fail
independently. PATH-050-C is the primary theorem witness: two
histories, identical on every local observable the recovery candidates
actually consult, opposite correct recovery actions, proven by applying
the identical naive procedure to independent clones of that identical
state and observing divergent real
effect counts (2 vs. 1).

Of the three candidates compared, **candidate C (local-only) loses** in
both directions the mission predicted: blind retry duplicates (A),
never-retry loses liveness (B). **Candidates A (provider-side
idempotency) and B (queryable durable receipt) both survive the tested
failure pattern** — at-most-once real effect under provider-side
idempotency for the tested retry pattern (D, E, F, G, H), and
recoverable completion under a queryable, assumed-trusted receipt (I,
with correct refusal to guess when no receipt exists at all, J). Neither
is claimed to be universally sufficient.

**CROSS-REVIEW CORRECTION:** PATH-050-K and PATH-050-K2 together show
that BOTH candidates — not candidate B alone — are conditional on an
unverified assumption: receipt authenticity. `EffectProvider::execute`'s
own duplicate-suppression check and `query_receipt` read the identical
`receipts` memory, so a forged or merely-wrong entry defeats either
recovery path identically. The precise, corrected claim is: safe
recovery, for the tested pattern, requires EITHER an authoritative
completion witness capable of distinguishing effect-happened from
effect-not-happened, OR an effect boundary whose retry semantics make
repeating the same canonical operation harmless — and the word
AUTHORITATIVE in the first branch is exactly what this candidate cannot
itself establish, for whichever surviving candidate is used to reach it.
A bare idempotency key is never itself evidence that completion
occurred; only the provider's own memory associated with that key is,
and that memory's trustworthiness is unverified here.

No generic exactly-once claim is made anywhere in this finding. The
precise claims are: at-most-once real effect under provider-side
idempotency for the tested retry pattern; recoverable completion under a
queryable, assumed-authoritative receipt (that assumption unverified for
either candidate that relies on it); and ambiguity remains, correctly
surfaced as `Unresolved` rather than guessed, whenever no additional
completion witness is available.

**Likely next PATH**: crash point C2 (receipt persistence itself as an
independently-failing step) and receipt-authenticity as its own
exogenous trust boundary are the two open threads this finding names
but does not close — either could be the seed of a follow-on mission,
alongside the untested question of concurrent recovery attempts racing
each other over the same stuck `Reserved(K)`.

No production code was modified. No payload-hash dedup, no
`contribution_id` heuristic, no database transactions, no HTTP calls,
and no distributed queue were used anywhere in the candidate.
