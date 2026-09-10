# FIND-027 — Durable Replay Memory Requires an Atomic Occurrence-Claim Boundary Under Concurrency

Status: OBSERVED / DERIVED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-049
Canonical prior commit: PATH-048 at `27ff331`

## Correction notice

Cross-review found one theoretical overclaim and three wording/scope
issues, all corrected below:

1. **Canonicalization-location overclaim.** An earlier revision said
   alias-race safety (PATH-049-F) works specifically because
   canonicalization executes INSIDE the locked critical section, and
   implied canonicalization outside the lock would allow the attack.
   Inspection of the test (`contract` is cloned per-thread, and
   `contract.authority_for` is a pure, read-only lookup touching no
   shared mutable state) shows this does not follow: F establishes that
   the synchronization/claim DOMAIN must converge on canonical occurrence
   identity, not raw `SourceId` identity — it does NOT establish that
   canonicalization must physically execute while holding the `Mutex`. A
   safe architecture could canonicalize outside the lock and claim only
   the resulting canonical key; two aliases would still converge on the
   same contention domain. Corrected below and in the test's own doc
   comment.
2. **"Linearizable is necessary" overstated.** The deterministic TOCTOU
   attack (A) proves separable stale check-then-license is unsafe; the
   `Mutex` candidate proves ONE serialized/indivisible transition is
   sufficient for the cases tested. Narrowed to the approved theorem: for
   the constructed claim-before-effect architecture, contenders for the
   same canonical occurrence require a single-winner claim boundary
   before economic permission is issued. The adopted candidate CAN be
   described as linearizable; this finding does not claim
   linearizability, as a general consistency model, is uniquely
   necessary — CAS/atomic RMW, database uniqueness/transactional
   constraints, optimistic concurrency with retry, and actor/queue
   serialization are uncompared alternative mechanisms in this
   experiment, not classified here as "weaker" or "non-linearizable."
3. **"No yield points" removed as causal justification.** Ordinary
   synchronous execution can be preempted at any instruction; the
   `Mutex` provides indivisibility by EXCLUDING other claimants from the
   protected section while one holds it, not because the protected code
   happens to contain no explicit yield points. Corrected below.
4. **D and J reframed as interleaving-state models, not concurrent
   tests.** Both are sequential: they correctly demonstrate the state
   another caller would observe between `Reserved` and `Settled`, but
   they do not themselves execute concurrent callers. The real-thread
   concurrency evidence is B/C/E/F/G/H/I. The Verdict's claim that the
   post-linearization property was "tested... across B, C, D, E, F, G,
   H, I, J" is also scoped precisely below: B/C/F genuinely contest the
   SAME occurrence; E/G intentionally exercise DIFFERENT occurrence
   keys; H/I exercise epistemic/positive-record boundaries; D/J exercise
   Reserved/Settled temporal state — these are four distinct property
   classes, not nine instances of one.

## Hypothesis under test (H20)

Sequential reserve-before-effect ordering (PATH-048's rule) is
insufficient for at-most-once settlement under concurrent calls. If
"check whether K exists" and "reserve K" are separable operations, two
concurrent callers can both observe K absent and both obtain a license
to apply the economic effect. A replay-safe concurrent settlement
boundary therefore requires some atomic/linearizable occurrence-claim
operation, or an equivalent serialization mechanism, over the canonical
`OccurrenceKey`.

## Amendment to H20

H20 as originally stated (above) names "atomic/linearizable" as one
open-ended category. The candidate tested supports a narrower, precise
theorem, not the broad category claim:

- **H20 (amended):** for the constructed claim-before-effect
  architecture, contenders for the same canonical occurrence require a
  single-winner claim boundary before economic permission is issued —
  two callers must not both cross absent → licensed based on the same
  pre-claim state.

This is confirmed. The broader reading of H20's original wording —
that "atomic/linearizable," as a general consistency-model category, is
the uniquely necessary shape of that boundary — is NOT established:
CAS/atomic RMW, database uniqueness/transactional constraints,
optimistic concurrency with retry, and actor/queue serialization are
uncompared alternative mechanisms in this experiment (see NOT
ESTABLISHED), not ruled out.

## Background

PATH-048 established durable cross-call identity memory
(`Path048Ledger`, keyed on `OccurrenceKey{namespace, event_id}`,
tracking `Reserved`/`Settled` state and `HistoryCompleteness`) and
explicitly did not test concurrent reservations, atomic check-and-set,
race conditions, or multi-node/distributed linearizability. PATH-049
isolates exactly that gap: with every epistemic prerequisite
(AUTHORITY, STABILITY, SEPARATION, `HistoryCompleteness`) held good,
does the candidate survive two resolution calls racing for the SAME
canonical occurrence?

## Candidate tested

Two candidates, deliberately contrasted:

- **Losing candidate — `Path049NaiveSplitLedger`**: existence check
  (`check`) and reservation (`reserve`) are two separate, independently
  observable method calls with nothing enforcing atomicity between them.
  Exists solely to demonstrate the race; never adopted.
- **Adopted candidate — `path_049_atomic_claim`**: a thin
  `std::sync::Mutex<Path048Ledger>` wrapper that calls PATH-048's own,
  UNMODIFIED `path_048_reserve` while holding the lock for its entire
  duration. No new resolution logic is introduced. What makes the "does
  K exist? if not, insert `Reserved(K)`" transition indivisible is that
  the `Mutex` EXCLUDES every other thread from entering that protected
  section while one holder is inside it — not the absence of explicit
  yield points in the protected code. Ordinary synchronous execution can
  still be preempted by the scheduler at any instruction; preemption
  inside the critical section is harmless here specifically because no
  OTHER thread can concurrently be executing that same section while the
  lock is held.

The theorem under test concerns **indivisibility of the claim
transition**, not `Mutex` specifically — `Mutex` is the smallest
mechanism this test-local candidate needed to obtain it.

## Losing interleaving (primary bug witness)

Forced deterministically, with no threads and no scheduler dependence
(`path_049_a_deterministic_toctou_race_yields_two_economic_licenses`):

```
CALL A: check(K) -> Absent
CALL B: check(K) -> Absent      (before A reserves)
CALL A: reserve(K)
CALL B: proceeds from its STALE Absent observation
```

Both A and B obtain an economic license for the same occurrence `K`,
even though each individually obeys PATH-048's sequential
reserve-before-effect ordering. At-most-once safety fails the moment
check and reserve are separable, independently observable steps —
confirmed executably, not merely argued.

## Adversarial cases, all passing (`cargo test --test pathological_cases path_049`, 10/10)

| Case | Attack | Result |
|---|---|---|
| A (primary witness) | deterministic TOCTOU: A checks Absent, B checks Absent, A reserves, B proceeds from stale check | both obtain a license — the bug, forced deterministically |
| B | same K, same payload, real concurrent threads (`Barrier`-synchronized start) | exactly 1 `New`, the other `Reserved` |
| C | same K, different payloads, concurrent | exactly 1 `New`, the other `Conflicted`; winning `Reserved` payload verified unchanged |
| D | Reserved-vs-Settled ordering: second delivery before confirmation, then confirm, then retry | pre-confirmation delivery gets `Reserved`; post-confirmation retry gets `AlreadySettled` |
| E | two DIFFERENT `event_id`s, same namespace, concurrent, one coarse whole-ledger `Mutex` | both acquire independently — execution serialized, semantics not |
| F | two raw `SourceId`s aliased to the SAME `NamespaceId`, same event_id/payload, concurrent | exactly 1 economic license total; naive raw-source_id-keyed lock shown directly to fail the same attack |
| G | identical event_id string, two DIFFERENT namespaces, concurrent | both acquire — event_id remains namespace-scoped, never globally unique |
| H | first sighting, `HistoryCompleteness = Unknown`, concurrent, perfect atomicity | both `Unresolved`; zero economic licenses; zero `Reserved` records created |
| I | pre-existing positive record, `HistoryCompleteness = Unknown` throughout, concurrent retries | both `AlreadySettled`; zero new licenses — completeness gates misses only |
| J | claim acquired, never confirmed, retry arrives | retry gets `Reserved`, not `New` — safety holds, liveness does not (extends PATH-048-J) |

Cases B, C, E, F, G, H, I use real `std::thread` + `std::sync::Barrier`
(no sleeps as correctness synchronization); B alone satisfies the "at
least one real thread test" requirement, and the remaining six add
further independent confirmation. All ten passed 25/25 consecutive runs
in local verification with no flakiness observed.

## OBSERVED

1. The deterministic interleaving (A) produces two economic licenses
   from a naive split check/reserve design, reproducibly and without any
   dependence on thread scheduling.
2. Every threaded case (B, C, E, F, G, H, I) counts economic licenses
   explicitly (verdicts equal to `New`), never merely a final map/record
   count — satisfying the mission's explicit distinction that "a map can
   end with one row after BOTH callers were wrongly licensed."
3. Case B: across the runs performed, exactly one thread's verdict is
   `New` and the other is `Reserved`, regardless of which thread's OS
   scheduler slot executes first.
4. Case C: exactly one thread acquires (`New`); the other observes
   `Conflicted`; the `Reserved` record's stored payload, read back after
   both threads join, equals the winning thread's own payload — no
   overwrite occurred.
5. Case F: canonicalization (via `TrustedSourceContract::authority_for`)
   happens inside `path_048_reserve`, before `records` is ever touched —
   exactly one license results despite two different raw labels, because
   the claim converges on the canonical `NamespaceId`. `contract` is
   cloned per-thread and `authority_for` is a pure, read-only lookup
   touching no shared mutable state, so this canonicalization step would
   resolve identically whether it executed before or after lock
   acquisition; the test does not establish that it must occur inside
   the lock, only that the CLAIM must be keyed on the canonical result. A
   raw-`SourceId`-keyed `HashSet` is shown directly, in the same test, to
   treat the two labels as separate keys and grant both a slot.
6. Case H: after both threads join, `guard.records.is_empty()` holds —
   no `Reserved` record was created by either caller, confirming the
   completeness gate blocks record creation entirely for this case, not
   merely the returned verdict.
7. Case D and J are SEQUENTIAL, deterministic interleaving-state models
   — they do not execute concurrent callers. Run through
   `path_049_atomic_claim` (PATH-048's own `path_048_reserve`
   unchanged), they directly construct the exact before/after-
   confirmation ordering a genuinely concurrent caller would encounter,
   reproducing PATH-048-J's Reserved-vs-Settled distinction exactly and
   connecting it to the concurrent model without themselves racing two
   threads.

## DERIVED

- **H20 is confirmed for the constructed race, with its scope narrowed
  to the amended theorem.** The naive split design fails exactly as
  predicted (A). The atomic candidate survives every attack constructed
  (B–J). Amended H20: for the constructed claim-before-effect
  architecture, contenders for the SAME canonical occurrence require a
  single-winner claim boundary before economic permission is issued —
  two callers must not both cross absent → licensed based on the same
  pre-claim state. The adopted `Mutex` candidate can be described as
  linearizable, but this experiment does NOT establish that
  linearizability, as a general consistency model, is the unique or
  minimal way to satisfy that boundary — only that check-then-insert
  must be indivisible from the perspective of concurrent callers
  contending for the same canonical key. CAS/atomic RMW, database
  uniqueness/transactional constraints, optimistic concurrency with
  retry, and actor/queue serialization are uncompared alternative
  mechanisms in this experiment (see NOT ESTABLISHED) — not classified
  here as weaker or non-linearizable, simply not constructed or tested.
- **The ADOPTED candidate's logical linearization point is the first
  successful transition of K from absent to `Reserved(K)`, inside one
  locked `path_048_reserve` call.** This is a property of the `Mutex`
  candidate specifically, not a claim that every possible correct
  coordination model must expose this exact implementation-level
  linearization point — an uncompared alternative mechanism (e.g. a
  database uniqueness constraint or a CAS retry loop) could satisfy the
  amended H20 theorem via a different mechanical shape entirely. For
  THIS candidate: after that point, no OTHER concurrent caller
  CONTENDING FOR THE SAME K can still legally receive `New` for it —
  this is exactly what B, C, and F demonstrate, the three cases that
  genuinely contest one occurrence, each showing exactly one `New` per
  contested key regardless of physical thread-execution order. The
  property held in every genuinely contested case constructed; no
  counterexample was found. (E and G deliberately test the converse,
  complementary property — that DIFFERENT canonical keys do NOT
  incorrectly serialize against each other — not an instance of this
  same-key property; see the adversarial-case scope note below.)
- **Atomicity does not substitute for epistemic admissibility — it is
  strictly downstream of it.** Case H shows that even a perfectly
  atomic, race-free claim mechanism still correctly refuses to settle an
  Unknown-completeness first sighting, and creates no side effect (no
  `Reserved` record) while refusing. Concurrency correctness and
  epistemic correctness are orthogonal properties this candidate keeps
  properly separated: fixing one does not paper over gaps in the other.
- **A single, coarse, whole-ledger lock does not impose incorrect
  cross-key semantics**, even though it does serialize EXECUTION of
  concurrent claims for different keys (E, G). This experiment does not
  show a coarse lock is necessary or optimal — only that it is
  sufficient and semantically correct for the cases constructed; nothing
  here evaluates contention cost or whether a finer-grained lock would
  behave identically under load.
- **The synchronization/claim domain must converge on CANONICAL
  occurrence identity, not raw `SourceId` identity — this is narrower
  than "canonicalization must execute inside the lock."** PATH-047/
  FIND-025 already established canonicalization-before-claim for the
  single-call case; F confirms the claim domain must still be the
  canonical key under concurrent execution — the raw-`SourceId`-keyed
  contrast in the same test shows a lock/key scoped to the wire label
  would fail this attack, because it would split one economic occurrence
  into two claim domains. This does NOT establish that canonicalization
  must physically execute while holding the `Mutex`: `authority_for` is
  a pure, read-only lookup against each thread's own cloned, immutable
  `TrustedSourceContract`, touching no shared mutable state, so it
  resolves identically regardless of when it runs relative to lock
  acquisition. A safe architecture could canonicalize outside the lock
  entirely and claim only the resulting canonical key; two aliases would
  still converge on the same contention domain. In this candidate,
  canonicalization happens to occur inside the locked call — an
  implementation detail of `path_048_reserve`, not a load-bearing
  requirement this test demonstrates.
- **Safety and liveness remain properly separated under concurrency, as
  they were under PATH-048's sequential crash model.** J shows an
  acquired-but-unconfirmed claim blocks every further economic license
  (safety holds) while never itself resolving (liveness does not) — the
  atomic candidate changes nothing about this distinction; it only
  changes how MANY concurrent callers can observe the stuck state
  without corrupting it (any number, all correctly told `Reserved`).

### The three claims this experiment does not collapse

1. **ATOMIC CLAIM** — exactly one caller can acquire the occurrence
   identity. **This is what PATH-049 tests, and it holds for every case
   constructed.**
2. **DURABLE CLAIM** — the acquired reservation survives the relevant
   crash boundary. PATH-048 gave sequential evidence about this
   ordering (reserve-before-effect); PATH-049 adds no new evidence here.
3. **ATOMIC ECONOMIC EFFECT** — the economic side effect and the
   settlement-state transition complete as one consistent unit. **NOT
   tested here.** Case J is the visible edge of this gap: an atomically
   acquired claim that is never confirmed is safe but permanently stuck,
   because nothing in this candidate makes "apply the economic effect"
   and "record `Settled`" one atomic unit — that is explicitly flagged
   below as the likely next research boundary, not something PATH-049
   solves.

## NOT ESTABLISHED

- **Whether a `Mutex` (or any single specific mechanism) is necessary,
  and whether linearizability specifically is the unique consistency
  model that works.** Only that SOME indivisible, single-winner claim
  boundary is necessary for the constructed race. The following are
  uncompared alternative mechanisms / coordination strategies in this
  experiment — not constructed, not run, and not classified as "weaker"
  or "non-linearizable," simply untested here:
  - CAS / atomic read-modify-write primitives
  - database uniqueness / transactional constraints
  - optimistic concurrency with retry
  - actor / queue-style serialization

  Whether any of these could preserve the same at-most-once
  economic-license invariant is explicitly open.
- **Per-ledger vs. per-namespace vs. per-`OccurrenceKey` synchronization
  granularity.** Every case here uses one whole-ledger `Mutex`. Whether
  a finer-grained lock is necessary at higher contention, or would
  behave identically under the same adversarial cases, is not tested.
- **Multi-node/distributed linearizability.** This candidate is a
  single-process, in-memory `Mutex`. Nothing here evaluates a
  distributed lock, a consensus protocol, or cross-node atomicity —
  H20's constructive form is deliberately scoped to "a single logical
  linearization point," not a distributed one.
- **Exactly-once delivery or exactly-once economic-effect completion.**
  Explicitly and repeatedly not claimed (see the three-claims
  distinction above). The gap between "claim atomically acquired" and
  "economic effect durably applied and confirmed as one unit" is real
  and unaddressed — **this is very likely the next PATH**: an
  ATOMIC-ECONOMIC-EFFECT boundary (claim #3 above), testing whether the
  side effect and the `Settled` transition can be made to succeed or
  fail together, and what happens to a claim whose effect application
  itself fails or crashes mid-flight.
- **Contention/performance behavior.** No case measures lock contention,
  throughput, or fairness among more than two concurrent callers; all
  cases use exactly two racing threads.
- **`ContractVersion` necessity** — still unread by any decision in
  `path_048_reserve`/`path_049_atomic_claim`; no PATH-049 test consults
  it. Unchanged from FIND-026: NOT ESTABLISHED.
- **`pruned_horizon`'s numeric `CallId` value** — PATH-049 does not
  exercise pruning at all (no case calls `prune_namespace`), so it adds
  no evidence either way about this field beyond FIND-026's existing NOT
  ESTABLISHED.
- **`ever_active_namespaces`** — confirmed to remain a convenience/cache
  only, in every state PATH-049 also constructs (see DERIVED in
  FIND-026); no PATH-049 test relies on it being decision-authoritative,
  and none needed to.
- Whether this synchronization design should be implemented in
  production (`src/contribution.rs`, `src/distribution.rs`) — out of
  scope; test-local only, per this mission's constraints.

## Verdict

**H20 CONFIRMED for the constructed race, narrowed to what was actually
tested.**

Sequential reserve-before-effect ordering alone is insufficient the
moment check and reserve are separable operations (PATH-049-A, the
primary bug witness, demonstrated deterministically with no scheduler
dependence). The amended H20 theorem: for the constructed
claim-before-effect architecture, contenders for the same canonical
occurrence require a single-winner claim boundary before economic
permission is issued — two callers must not both cross absent →
licensed based on the same pre-claim state. The ADOPTED candidate
satisfies this via an indivisible occurrence-claim transition with its
own logical linearization point — the first successful `Reserved(K)`
insertion inside one locked call — after which no OTHER caller
contending for THE SAME `K` may still receive `New`. This specific
linearization point is a property of the `Mutex` candidate, not a claim
that every correct coordination model must expose this exact
implementation-level shape.

This property is not uniformly "tested by B–J": the nine cases fall into
four distinct classes, and each was tested only for what it actually
exercises. **B, C, F** genuinely contest the SAME canonical occurrence
and directly test the single-winner linearization property (same
payload, different payload, and alias convergence, respectively). **E,
G** intentionally exercise DIFFERENT canonical occurrence keys and test
the converse, complementary property — that the claim mechanism does
NOT incorrectly serialize across keys. **H, I** exercise epistemic/
positive-record boundaries — that atomicity does not launder missing
completeness into certainty, and that a positive hit remains usable
under Unknown completeness regardless of racing. **D, J** are sequential
interleaving-state models (not concurrent-thread tests) that connect
PATH-048's Reserved-vs-Settled temporal semantics to the concurrent
model. No case is claimed to independently re-test the same-key
single-winner property that only B/C/F actually exercise.

This finding does NOT establish that a `Mutex` specifically, or any
particular granularity of locking, is the unique or minimal correct
mechanism, and does NOT establish that linearizability, as a general
consistency model, is uniquely necessary — only that indivisibility of
the claim transition is necessary for the constructed race, and that one
coarse, whole-ledger `Mutex` (describable as linearizable) around
PATH-048's unmodified `path_048_reserve` is sufficient for every case
constructed. CAS/atomic RMW, database uniqueness/transactional
constraints, optimistic concurrency with retry, and actor/queue
serialization are uncompared alternative mechanisms in this experiment —
not constructed, not tested, and not classified here as weaker or
non-linearizable. It does NOT claim distributed consensus, exactly-once
delivery, or atomicity of the economic effect itself — the
atomic-economic-effect boundary (claim #3 in the three-claims
distinction above) remains open and is flagged as the most likely next
research direction.

No production code was modified. No payload-hash dedup and no
`contribution_id` heuristic were used anywhere in the candidate.
