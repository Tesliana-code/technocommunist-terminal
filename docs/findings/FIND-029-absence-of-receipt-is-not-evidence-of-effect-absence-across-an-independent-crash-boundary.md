# FIND-029 — Absence of Provider Completion Memory Does Not Establish Absence of Economic Effect Across an Independent Persistence Gap

Status: OBSERVED / DERIVED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-051
Canonical prior commit: PATH-050 at `da3afb5`

Result: provider-side idempotency/receipt memory fails to protect a
retry if the real effect can commit before that memory durably records
the operation — moving replay memory from the local ledger (PATH-048)
to the provider (PATH-050) relocates the temporal ambiguity rather than
removing it.

## Hypothesis under test (H22)

A provider-side receipt or idempotency store cannot guarantee safe
retry if the real effect can commit before the provider's completion
memory commits. If `effect(K)` succeeds but `completion_memory[K]` is
absent after a crash, a retry may be observationally identical to a
world in which `effect(K)` never happened. If provider retry semantics
depend only on that missing completion memory, retrying `K` can
duplicate the real effect. Candidate closures: an effect/completion
atomic failure domain, intrinsically retry-idempotent effect semantics,
authoritative reconciliation against effect truth, or trusted/
authoritative completion evidence with adequate durability. A stable,
canonical operation identity is an enabling key these closures depend
on, not itself a closure (PATH-051-E/G).

## Background

PATH-050 established that safe recovery, for its tested failure
pattern, required either provider-side idempotency or a queryable
receipt — and that both trusted the same provider `receipts` memory
(FIND-028, corrected under cross-review to note this applies to BOTH
surviving candidates, not one). PATH-050's fake provider modeled "the
real effect executes" and "completion memory persists" as ONE atomic
step (`execute()`), so it could not expose what happens if they fail
independently. PATH-051 attacks exactly that simplification, which
FIND-028 explicitly flagged as crash point C2, unmodeled.

## Provider model

`Path051SplitProvider` replaces PATH-050's atomic `EffectProvider` with
two separately-callable steps:

- `execute_real_effect(operation_id, payload)` — the real effect
  commits. Touches ONLY `ground_truth`.
- `persist_completion(operation_id, payload)` — completion memory
  commits. Touches ONLY `completion_memory`.
- `query_completion(operation_id)` — read-only, `completion_memory`
  only.

A test simulates a crash between the first two steps simply by not
calling the second. `EffectOperationId` remains the canonical
`OccurrenceKey` (via `path_048_occurrence_key`), unchanged from
PATH-047/048/050.

## Ground truth vs. recovery-visible state

This distinction is the methodological center of the experiment and is
stated explicitly rather than left implicit:

| Field | Classification | Who may read it |
|---|---|---|
| `GroundTruthEffectLog.real_effect_count` | **GROUND TRUTH ONLY** | This experiment's own test assertions, after recovery has already run — never any recovery function |
| `CompletionMemory.receipts` (via `query_completion`) | **RECOVERY-VISIBLE** | The only provider state any recovery function in this file reads |
| `Path048Ledger.records` (via `ledger.records.get(key)`) | **RECOVERY-VISIBLE** | Local durable state, unchanged from PATH-048/050 |

Audited directly (grep-verified, not merely asserted): `path_051_recover`
and `path_051_retry_idempotent_by_completion_memory` reference
`.query_completion()`, `.execute_real_effect()` (write, return value
`()`, nothing to branch on), and `.persist_completion()` (write) —
neither function contains any reference to `.ground_truth` anywhere in
its body. `real_effect_count` exists purely as this experiment's
omniscient observation window; a real crashed process's recovery code
could no more consult it than this candidate's functions do.

## Crash-point matrix

| Point | What has committed | Recovery-visible evidence |
|---|---|---|
| P0 | nothing — before the real effect | `query_completion` → `None` |
| P1 | REAL EFFECT committed; completion memory NOT persisted | `query_completion` → `None` — **IDENTICAL to P0** |
| P2 | completion memory persisted (local `Settled` still pending) | `query_completion` → `Some(payload)` |
| P3 | local `Settled` persisted (PATH-048, unchanged) | `AlreadySettled` |

P0 and P1 are the crucial pair: different ground truth (effect never
ran vs. effect genuinely ran), identical recovery-visible evidence.
This is PATH-048's ABSENCE != NOVELTY recurring at the external effect
boundary — **ABSENCE OF RECEIPT != ABSENCE OF EFFECT**.

This corresponds precisely to what FIND-028's crash matrix named C1 vs.
C2 and explicitly declined to model independently (PATH-050's atomic
`execute()` could not be interrupted between effect and receipt). P0/P1
is C1/C2 built and executed.

## Adversarial cases, all passing (`cargo test --test pathological_cases path_051`, 11/11)

| Case | Attack | Result |
|---|---|---|
| A | real effect succeeds (P1), crash before completion persists, naive retry | `real_effect_count == 2` — primary concrete failure witness |
| B | crash before effect (P0); same observable state as A | retry correctly executes once, `real_effect_count == 1` |
| C | WORLD A (P1) vs. WORLD B (P0), identical recovery-visible evidence | same procedure yields opposite ground-truth outcomes (2 vs. 1) — core theorem |
| D | completion persists (P2) before local `Settled` crash | receipt found, no second effect, local settlement reconstructed |
| E | operation_id stable across original delivery and retry, completion memory lost | duplicates anyway — stability necessary, not sufficient |
| F | changed payload after completion memory lost | `Conflicted`, caught locally before the provider is even consulted |
| G | alias convergence (same operation_id via two raw labels), completion memory lost | duplicates anyway — canonical identity necessary, not sufficient |
| H | K1 (completion persisted) and K2 (completion lost) | fully independent outcomes; no cross-key coalescing or poisoning |
| I | completion record exists, effect never happened (forged — mirrors PATH-050-K/K2) | needed effect suppressed — false positive |
| J | effect happened, completion record absent (mirror of I) | effect duplicated — false negative, the central new result |
| Concurrent probe | two unsynchronized recovery workers, missing completion memory | both execute — confirms the obvious, not expanded into a full study |

## OBSERVED

1. **PATH-051-C, proven executably.** `ledger_a.records == ledger_b.records`
   and `provider_a.query_completion(&key) == provider_b.query_completion(&key)`
   (both `None`) hold by direct assertion. The identical recovery
   procedure, applied to independent clones, produces
   `ground_truth.real_effect_count` 2 (World A) vs. 1 (World B) —
   `assert_ne!` holds. Recovery's own code was grep-audited to contain
   no reference to `.ground_truth`.
2. PATH-051-A: `provider.execute_real_effect` is called once (ground
   truth = 1), `persist_completion` is never called, `query_completion`
   returns `None`, and `path_051_recover` then executes the effect
   again — `ground_truth.real_effect_count == 2` asserted directly.
3. PATH-051-E: `key_original == key_retry` asserted (operation_id IS
   stable), yet `real_effect_count` still reaches 2 — the duplicate is
   attributable specifically to lost completion memory, with operation
   identity held constant and confirmed unchanged.
4. PATH-051-G: `key_a == key_b` asserted (aliases converge on one
   operation_id, exactly as PATH-047/048/050 established), yet the
   duplicate still occurs — canonicalization is confirmed necessary
   (without it, F-style key-splitting would ALSO defeat recovery) but
   proven insufficient on its own here.
5. PATH-051-H: `real_effect_count == 3` after processing both
   occurrences (K1 once, K2 twice) — the exact arithmetic confirms K1's
   clean completion record was neither consulted for nor corrupted by
   K2's gap.
6. PATH-051-I and PATH-051-J, placed adjacently and using the identical
   assertion structure, make the false-positive/false-negative mirror
   explicit: I suppresses a needed effect (`real_effect_count` stays at
   0 despite `AlreadyComplete` being reported); J duplicates a genuine
   one (`real_effect_count` reaches 2 despite the record legitimately
   having existed in ground truth, just not in recovery-visible
   memory).
7. The concurrent-recovery probe: two sequential, unsynchronized
   `query_completion` calls both return `None` (forced deterministically,
   not via thread scheduling), and both subsequent
   `execute_real_effect` calls proceed, yielding
   `real_effect_count == 2` from a single missing-completion-memory
   starting state.

## DERIVED

- **H22 is confirmed for the constructed failure pattern.** A queryable,
  durable-after-persistence completion store does not, merely by having
  those two properties, guarantee safe retry. The missing property is
  ordering: whether the store's record becomes durable no later than
  the effect it describes. PATH-051 tests a provider where it does not,
  and shows the ambiguity this creates is exactly PATH-048's ABSENCE !=
  NOVELTY, recurring at a new boundary: ABSENCE OF RECEIPT != ABSENCE OF
  EFFECT.
- **Completion memory must be authoritative in two independent
  directions to be trusted alone, and this candidate's memory achieves
  neither.** Existing (positive) completion evidence must correspond to
  reality — PATH-050-K/K2 and PATH-051-I show it need not (soundness
  gap). Absent (negative) completion evidence must mean what recovery
  assumes it means — PATH-051-A/C/E/G/J show it need not either
  (coverage gap). These are genuinely different failure directions with
  different consequences: a soundness failure suppresses a needed
  effect; a coverage failure duplicates one. Deliberately not called
  "completeness" here, to avoid conflating this coverage property with
  PATH-048's distinct `HistoryCompleteness` type — the SHAPE of the
  requirement (an explicit, exogenous, unverified attestation that
  negative evidence means what recovery assumes) is the same pattern
  recurring a third time (FIND-019 → FIND-026/048 → here), not the same
  mechanism.
- **In this candidate, operation-identity stability and canonicalization
  are necessary for completion memory lookups to converge on the
  intended record, but neither is sufficient once completion persistence
  can fail independently.** `path_051_recover` and
  `path_051_retry_idempotent_by_completion_memory` index
  `CompletionMemory` by canonical `OccurrenceKey` — that indexing choice
  is why a stable, canonicalized key is required for THIS candidate's
  lookups to converge at all. E isolates operation_id stability as the
  controlled variable; G isolates canonical alias convergence. Both hold
  exactly as PATH-047/048/050 established, and both scenarios still
  duplicate. This is not a contradiction of those prior findings — it
  is evidence that they answer a different question (which key to use)
  than the one PATH-051 asks (whether the memory behind that key is
  durable relative to the effect). No claim is made that every
  conceivable external receipt/reconciliation system must use this same
  identity scheme — only that this candidate's does, and that doing so
  is not enough.
- **The three-failure-domain distinction from FIND-027 needed one more
  split.** FIND-027 separated CLAIM COMMIT / EFFECT COMMIT / SETTLEMENT
  COMMIT. PATH-051 shows EFFECT COMMIT itself has an internal boundary
  when a provider maintains its own completion memory: the effect
  committing and that provider's OWN record of it committing are not
  the same event unless something makes them so. This is not a fourth
  top-level domain — it is EFFECT COMMIT's internal structure, exposed
  once the provider is modeled honestly.
- **A local write-ahead marker does not close the gap — reasoned only,
  not built.** "Persist 'about to execute K', then execute" introduces
  two MORE independently-crashable steps (write-ahead marker persisted;
  effect executed) with the identical shape of ambiguity between them —
  crash after the marker but before the effect looks identical, from
  later recovery's perspective, to crash after the effect but before a
  "done" marker. This is not demonstrated by an executed test in this
  experiment (per the mission's explicit instruction to keep unbuilt
  reasoning in DERIVED, not OBSERVED) — it is a structural argument: the
  ambiguity is a property of "two independently-crashable steps sharing
  one intended outcome," and adding a third step does not remove that
  property, it relocates which PAIR of steps is ambiguous.
- **Concurrent recovery without exclusion obviously fails, and a Mutex
  fixing it would be orthogonal to PATH-051's actual subject.** The one
  probe test confirms two unsynchronized workers can both execute from
  one missing-completion-memory state. A PATH-049-style Mutex around the
  check-then-act sequence would close THIS specific concurrent variant
  (exactly as PATH-049 closed the claim-race problem) — but it would not
  touch the SINGLE-worker crash ambiguity (P0 vs. P1) that is this
  finding's actual subject: a Mutex serializes concurrent ACCESS, it
  does not make one worker's two sequential steps atomic with respect to
  that same worker crashing between them.

## NOT ESTABLISHED

- **That a real provider's completion memory actually exhibits this
  persistence gap.** H22's premise (effect and completion memory can
  fail independently) is the experiment's assumed starting condition,
  per the mission brief — not derived from any real system. This
  repository has no real external effect provider to inspect.
- **Candidate B (an intrinsically idempotent effect, e.g. "set" rather
  than "add" semantics).** Discussed only. Modeling it honestly would
  require simulating converged application-level state independent of
  call count, which this experiment does not build.
- **Candidate C (authoritative external reconciliation against the
  effect system's own source of truth).** Discussed only. A real
  reconciliation channel is conceptually distinct from this experiment's
  `ground_truth` (which is a test-only oracle, not something a real
  recovery process could query) — building it honestly either collapses
  toward candidate A (if reconciliation is always available and
  authoritative) or requires machinery out of this mission's scope.
- **Candidate D (a transactional resource committing both together).**
  The same shape as candidate A, generalized; not built, per the
  explicit constraint against distributed transaction machinery.
- **Whether P1 is closable at all without external atomicity or an
  external contract.** No candidate built in this experiment closes it.
  This is recorded as a legitimate, strong result — "we found the
  boundary; no local protocol crosses it without a stronger external
  contract" — not a gap left open by insufficient effort.
- **A full concurrent-recovery study.** The one small probe test
  confirms the obvious failure mode under no exclusion; a rigorous,
  PATH-049-style study (real threads, exactly-one-license-style
  counting, a Mutex around the completion-memory check-then-act
  sequence) was deliberately not built, per the mission's explicit
  scope constraint. Flagged as the most likely next PATH.
- **Write-ahead marker mechanics**, beyond the structural argument in
  DERIVED — no executed test constructs or exercises one.
- **`ContractVersion`, `pruned_horizon`'s `CallId` value,
  `ever_active_namespaces`'s redundancy, and receipt-authenticity
  mechanisms** — all unchanged from FIND-026/027/028 and not newly
  addressed here; no case in this experiment exercises pruning, contract
  evolution, or an authenticity mechanism beyond re-confirming (I) that
  the forgery vulnerability survives the split model unchanged.
- Whether any of this design should be implemented in production
  (`src/contribution.rs`, `src/distribution.rs`) or integrated with a
  real database, HTTP effect provider, or queue — explicitly out of
  scope.

## Verdict

**H22 CONFIRMED for the constructed failure pattern.**

PATH-051-C is the primary theorem witness: two provider histories,
identical on every observable `path_051_recover` actually reads
(`ledger.records`, `query_completion`), opposite ground truth, opposite
correct recovery action — proven by applying the identical procedure to
independent clones of that identical evidence and observing divergent
real effect counts (2 vs. 1), with recovery logic grep-audited to never
read the ground-truth oracle used to grade it.

**ABSENCE OF RECEIPT != ABSENCE OF EFFECT is supported directly**, by
A, C, E, G, and J. Missing completion memory defeats safe AUTOMATIC
COMPLETION for both PATH-050 recovery shapes once persistence is
allowed to lag the real effect — but not in the same way, and PATH-051
only rebuilt and tested one of the two under the split model:

- **Execute-based retry (PATH-050's candidate A, the only shape
  PATH-051 rebuilt as `path_051_recover` /
  `path_051_retry_idempotent_by_completion_memory`)**: on absent
  completion memory it proceeds to call `execute_real_effect` anyway —
  this is what A, C, E, G, and J directly demonstrate duplicating the
  real effect. **Safety failure**: a real effect can be duplicated.
- **Query-only receipt recovery (PATH-050's candidate B,
  `path_050_recover_via_receipt_query`)**: by construction it never
  calls `execute()`/`execute_real_effect` — on absent receipt it returns
  `Unresolved` and performs no effect (confirmed by its unmodified
  PATH-050 code, `match provider.query_receipt(key) { ... None =>
  Path050RecoveryVerdict::Unresolved }`). PATH-051 did not rebuild this
  shape under the split provider, so no PATH-051 case exercises it
  against an independently-lost completion memory; by the same
  construction that made it safe under PATH-050's atomic provider, it
  cannot itself duplicate the effect under the split model either — it
  can only fail to complete. **Liveness/completion failure, not (on
  this evidence) a safety failure**: the record stays `Unresolved` and
  no second effect is caused.

Both shapes remain equally exposed to the SOUNDNESS gap already
established in FIND-028/PATH-050-K (a forged or otherwise incorrect
*positive* receipt is trusted by either shape without verification) —
that vulnerability is shared for the reason FIND-028 gives. The
COVERAGE gap this finding adds (what absent completion memory licenses
recovery to do) is where the two shapes diverge: it turns into a
duplicate for the execute-based shape and into a safely-stuck
`Unresolved` for the query-only shape.

**Completion memory would need to be authoritative in two independent
directions** — soundness of positive evidence (PATH-050-K/K2,
re-confirmed under the split model as I) and coverage of negative
evidence (this finding's central new result, J) — and this candidate's
tested memory achieves neither once effect and persistence commit
independently.

**Smallest surviving external contract tested**: none, honestly. Every
candidate that would close P1 (atomic effect+completion, an
intrinsically idempotent effect, authoritative external reconciliation,
a transactional resource) either already existed and was not rebuilt
(PATH-050's atomic candidate) or was discussed and deliberately not
constructed, because building it honestly would require either
reintroducing the exact atomicity this experiment set out to remove, or
machinery outside this mission's scope. P1 is left `Unresolved` for the
pure split/receipt-only design — a legitimate, strong negative result,
not an unfinished one.

No generic exactly-once claim, distributed atomic commit claim,
consensus-necessity claim, or claim about real payment-provider
behavior is made anywhere in this finding. No production code was
modified.
