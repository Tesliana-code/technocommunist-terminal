# PKV Transaction / Recovery Literature Map 001

Status: research note only. No production code, test code, or protocol
finding was modified to produce this document. No claim here is a new
FIND or PATH result — it is a map from FIND-025..029's already-tested
results onto established transaction/recovery literature, done before
choosing PATH-052.

## 1. Current protocol boundary

- **FIND-025**: `(source_id, event_id)` identity is sufficient for
  replay-safe pairwise/single-batch resolution only jointly with an
  explicit, exogenous trust fact (AUTHORITY, plus SEPARATION or
  STABILITY as the claim requires) — never from the key alone. Absent
  that fact, resolution is `Unresolved`/`Conflicted`, never guessed.
- **FIND-026**: Durable cross-call state is necessary (the Temporal
  Identifiability Theorem: identical current-event input can require
  different correct actions depending only on past state) and is
  sufficient only under an explicit, itself-unverifiable
  `HistoryCompleteness` attestation governing every negative
  occurrence-history lookup.
- **FIND-027**: Under concurrency, durable replay memory requires an
  atomic, single-winner occurrence-claim boundary — an indivisible
  `Reserved(K)` insertion with its own linearization point — because
  sequential check-then-reserve ordering alone is insufficient once
  check and reserve are separable operations.
- **FIND-028**: `Reserved(K)` alone (local claim state, no completion
  witness) cannot safely recover an external economic effect once
  effect execution and the durable `Settled` transition can fail
  independently; safe recovery for the tested pattern requires either
  provider-side idempotency or a queryable durable receipt, and both
  are conditional on an unverified assumption that the provider's
  receipt/idempotency memory is itself authentic (a soundness gap,
  PATH-050-K/K2).
- **FIND-029**: Provider-side completion memory can itself lag the real
  effect (a crash between `execute_real_effect` and
  `persist_completion`) — absence of receipt is not evidence of absence
  of effect (P0/P1: identical recovery-visible state, divergent ground
  truth). Execute-based recovery can duplicate the effect (safety
  failure); query-only recovery, by construction, cannot duplicate and
  instead stalls `Unresolved` (liveness/completion failure) when
  completion memory is absent.

## 2. Literature mappings

### Gray & Reuter — *Transaction Processing: Concepts and Techniques* (1993)

| Concept | Definition (concise) | Closest FIND/PATH | Fit | What it teaches us |
|---|---|---|---|---|
| Atomicity | A transaction's effects are all-or-nothing; no observer ever sees a partial result. | FIND-028/029's crash boundary | Exact fit, as a description of what is *missing* | PATH-050/051's split provider is precisely a provider where "the real effect" and "the completion record" are not atomic. Our "effect/completion atomic failure domain" candidate closure is literally Gray/Reuter atomicity applied across a provider boundary. |
| Write-ahead logging (WAL) | Durably log the intended change before applying it, so redo/undo can reconstruct a consistent state after a crash. | FIND-029 DERIVED's "write-ahead marker" discussion | Partial fit | WAL closes atomicity *within one resource manager's own log+data pair*. Our provider boundary has no shared log between the effect and its own receipt — this is exactly why FIND-029 DERIVED concludes a local write-ahead marker "relocates which pair of steps is ambiguous" rather than closing the gap. |
| In-doubt transaction | After a coordinator crash mid-2PC, a participant that has voted but not received a decision must block or heuristically resolve; its own state cannot reveal the true outcome. | P1 (FIND-029) | Analogous, not identical | Closest named term for "recovery-visible state cannot distinguish two different ground truths." Not identical: a 2PC in-doubt participant *did* durably record that it was mid-protocol (it voted). Our provider never ran a protocol at all — nothing is durably recorded between P0 and P1, so there is no vote to be "in doubt" about; the ambiguity is more primitive than 2PC in-doubt state. |
| Exactly-once / at-most-once reasoning | "Exactly once" is not attainable at the message layer; it is approximated at the application layer via durable idempotent identifiers checked against state. | FIND-025/026's OccurrenceKey pattern; FIND-029's explicit no-exactly-once disclaimer | Exact fit | Confirms our findings' existing discipline of never claiming exactly-once — this is the standard reason why, stated in the literature we're checking against. |

### Bernstein, Hadzilacos & Goodman — *Concurrency Control and Recovery in Database Systems* (1987)

| Concept | Definition | Closest FIND/PATH | Fit | What it teaches us |
|---|---|---|---|---|
| Serializability | Concurrent execution is correct iff equivalent to some serial order of the same transactions. | FIND-027's single-winner claim boundary | Partial fit (degenerate case) | FIND-027 never modeled general multi-object transactions; its single-winner property is the one-object (single key) write-write conflict case of serializability, not general serializability. No fit beyond that narrow instance. |
| Recovery under concurrent access (redo/undo via a shared durable log, checkpoints, LSNs) | Restart recovery reconstructs a consistent state by replaying/undoing log records against durable data, using the log as the single source of truth for what committed. | `Path048Ledger`; `CompletionMemory` | No fit at the mechanism level | We never built a shared LSN-ordered log, checkpoints, or a redo/undo distinction — our ledger is a plain in-memory map, not a durability mechanism in this book's sense. The relevant lesson is structural, not mechanical: classical recovery works because the log and the data it describes share one failure domain. FIND-029's whole result is that our provider's "log" (completion memory) and its "data" (the real effect) do *not* share one failure domain. |

### Kleppmann — *Designing Data-Intensive Applications* (2017)

| Concept | Definition | Closest FIND/PATH | Fit | What it teaches us |
|---|---|---|---|---|
| Linearizability | Every operation appears to take effect atomically at some point between its invocation and completion, consistent with real-time order. | FIND-027's "own logical linearization point" | Exact fit | This is a direct, already-correctly-scoped borrow — FIND-027 itself disclaims that this is a property of the tested `Mutex` candidate, not a universal claim. Nothing to correct here; the literature confirms the vocabulary was used precisely. |
| Idempotency keys / retry deduplication (ch. 8–9) | Client-generated request ID, deduplicated server-side against durable state, to make at-least-once delivery safe. | FIND-025..029's `OccurrenceKey` / `EffectOperationId` pattern; FIND-029's H22 | Exact fit | Kleppmann explicitly notes the server-side dedup table can itself be lost or fail to persist before the effect it guards — this is arguably the single closest literature statement of FIND-029's H22 found in this survey. |
| Fencing tokens / lease-based locks (ch. 8) | A monotonically increasing token issued with a lock, so a resource can reject actions from a holder whose lease has since expired. | Section 5 below (concurrent recovery) | Analogous, relevant to next PATH | Not tested by anything built so far; a candidate mechanism for the concurrent-recovery race, not something FIND-027 or FIND-029 used. |

### Pat Helland — "Life Beyond Distributed Transactions: An Apostate's Opinion" (CIDR 2007)

| Concept | Definition | Closest FIND/PATH | Fit | What it teaches us |
|---|---|---|---|---|
| Entity-local transaction boundary vs. messaging between autonomous scopes; at-least-once + idempotence as the realistic contract instead of a joint distributed transaction | Two independently-failing scopes generally cannot be made transactional together; the practical answer is idempotent messages plus reconciliation, not a stronger atomic primitive. | FIND-028/029's local-Reserved(K)-vs-external-provider split | Exact fit, as a framing | This paper is the closest prose statement of our entire experimental arc's conclusion: `Reserved(K)` (local scope) and the provider's real effect (external scope) are not one transactional scope, and no purely local protocol change closes that. Helland's own answer — idempotent messages plus reconciliation — is why "authoritative reconciliation against effect truth" belongs in H22's candidate-closure list as a first-class option rather than an exotic one. |

### Pat Helland — "Immutability Changes Everything" (CIDR 2015)

| Concept | Definition | Closest FIND/PATH | Fit | What it teaches us |
|---|---|---|---|---|
| Facts as immutable, versioned records; mutable/derived views computed by replaying an append-only fact log; reconciliation operates over facts, not over mutable state directly. | `GroundTruthEffectLog` vs. `CompletionMemory` split | Analogous, not identical | Our split is loosely in this spirit, but `CompletionMemory` is a plain overwritable `HashMap`, not an immutable/append-only fact store, and `GroundTruthEffectLog` is a test-only oracle, not a real fact log a recovery process could query. This suggests a concrete *design idea* for a stronger "trusted/authoritative completion evidence" candidate (an append-only completion-fact log rather than one mutable receipt) — but that idea has not been built or tested here. |

### Lamport — "Time, Clocks, and the Ordering of Events in a Distributed System" (1978)

| Concept | Definition | Closest FIND/PATH | Fit | What it teaches us |
|---|---|---|---|---|
| Happens-before / logical clocks; partial order of events across independent processes. | The concurrent-recovery probe (FIND-029's secondary case) | Mostly no fit for the core P0/P1 result; partial fit for the concurrent probe | Our core P0/P1 ambiguity exists with a *single* process and a *single* later observer — it is logically prior to any cross-process ordering question, so happens-before does not resolve it. It is only relevant background for two *concurrent* recovery workers racing over the same missing evidence, which is an interleaving/mutual-exclusion question, not something this paper's vector-clock machinery is needed to state. |

### Gray & Lamport — "Consensus on Transaction Commit" (2004/2006)

Read for boundary-setting only, per the mission brief; not a source to imitate.

| Concept | Definition | Closest FIND/PATH | Fit | What it teaches us |
|---|---|---|---|---|
| Paxos Commit / 2PC as consensus over a set of faithfully-participating, vote-logging resource managers. | N/A — no candidate in this repo | No fit as a mechanism to adopt | This paper solves a different problem than ours: coordinating a *set* of participants that each durably record their own vote around one joint decision. Our provider never runs a prepare phase and never votes; it performs an uncoordinated two-step write. Useful only to show what a *real* fix would require if the effect provider were modeled as a cooperating participant — which is explicitly out of scope (no distributed transaction machinery). |

## 3. P0/P1 ambiguity: closest classical model(s)

- **OBSERVED IN OUR TESTS**: P0 (effect never executed) and P1 (effect
  executed, completion memory not yet persisted) are identical on every
  observable `path_051_recover` reads (`query_completion` → `None` in
  both), and produce divergent `real_effect_count` (PATH-051-C).
- **KNOWN FROM LITERATURE**: the abstract shape — "a write committed,
  but a *separate* write recording that it committed did not, because
  they are two different writes" — is a standard write-ordering/
  durability problem in crash-recovery theory (Gray/Reuter; Bernstein/
  Hadzilacos/Goodman's redo/undo recovery), and Helland's framing gives
  the correct high-level name for *why* it exists here specifically: the
  effect and its receipt are in two different, non-transactional scopes.
- **ANALOGOUS BUT NOT IDENTICAL**: "in-doubt transaction" (2PC) is the
  closest single named term, but it presumes a resource manager that
  itself durably recorded entering the protocol (it "voted"); our
  provider records nothing before `execute_real_effect` returns, so
  there is no vote to be in doubt about — the ambiguity is more
  primitive than 2PC in-doubt state, not a special case of it.
- **STILL OPEN IN OUR MODEL**: no single classical theorem names this
  exact configuration precisely (an effect and a receipt-about-that-
  effect, both external to the recovering process, coupled by nothing
  but two sequential provider-side writes). The closest composite
  description is "at-least-once effect execution with no shared durable
  log between the effect and its own receipt" — Helland's framing plus
  Gray/Reuter's atomicity vocabulary, not one named result.

## 4. Candidate closure mechanisms

| Mechanism | Solves safety? | Solves liveness? | Merely relocates uncertainty? | Requires a stronger external contract? |
|---|---|---|---|---|
| Atomic transaction (effect + completion, one failure domain) | Yes | Yes | No | Yes — the provider itself must offer transactional coupling; PATH-050's atomic candidate already models this and was deliberately not rebuilt under the split provider. |
| Idempotent external effect ("set" not "add" semantics) | Yes (duplicates become no-ops) | No — does not by itself let recovery *confirm* whether the effect happened, only makes retrying harmless | Sidesteps the observability question rather than closing it | Yes — the provider must offer this semantic; not built here (FIND-029 NOT ESTABLISHED, Candidate B). |
| Authoritative reconciliation against effect truth | Yes, if reachable | Yes, if reachable | No, if it truly exists and is trusted | Yes — the strongest contract of the set: an external, authoritative oracle we do not have (FIND-029 Candidate C, discussed only). |
| WAL / local prepare-intent marker | No | No | Yes — adds a third independently-crashable step; relocates which *pair* is ambiguous (FIND-029 DERIVED, reasoned only) | Only closes the gap if marker+effect are themselves made atomic, at which point it has become the "atomic transaction" case above, not "just WAL." |
| 2PC / transaction manager | Yes, in theory | Yes, in theory | No, if the protocol completes | Yes — requires the provider to be a cooperating participant that durably records its own vote; per Helland, generally unavailable for real external effect providers, and explicitly out of scope here (Gray & Lamport read for boundary-setting only). |
| Saga / compensating action | N/A — does not address detection | N/A | Orthogonal — presupposes you already know a step completed, which is exactly what P0/P1 does not tell you | N/A — wrong tool for H22; solves a different problem (undoing known-completed multi-step workflows). |
| Fencing / single-winner recovery | N/A for the single-worker P0/P1 gap | N/A for the single-worker P0/P1 gap | N/A | Solves a *different* race — concurrent recovery workers, not the single-worker crash ambiguity (FIND-029 DERIVED already states a Mutex here "would not touch the single-worker crash ambiguity"). This is the right tool for Section 5, not for H22 directly. |

## 5. Concurrent recovery

FIND-029's one concurrent-recovery probe (two unsynchronized workers,
missing completion memory, both execute) is exactly the race named in
research question D. Established concepts that map onto it:

- **Mutual exclusion / locking** (Bernstein/Hadzilacos/Goodman) — the
  general classical tool for "two actors, one contested decision."
- **Leases and fencing tokens** (Kleppmann, ch. 8) — needed once a lock
  holder can appear to have failed (paused, slow, crashed) and a second
  worker must safely take over without the first worker's late action
  landing after handoff.
- **Single-winner claim linearization** (already established locally,
  FIND-027) — but FIND-027's `Mutex` governs the *original claim* step
  (`Reserved(K)` insertion). It was never tested governing the
  *recovery* decision specifically — i.e., two workers both finding
  `Reserved(K)` already present, `query_completion` absent, and racing
  on whether to call `execute_real_effect`. That is a narrower, more
  precise race than FIND-027 covered, and it is exactly what FIND-029's
  DERIVED section flags as the most likely next PATH.

## 6. What NOT to overclaim

- This literature review does not prove any classical theorem was
  formally reproduced by our tests — it maps vocabulary, nothing more.
- 2PC's "in-doubt transaction" is **analogous**, not **identical**, to
  our P1 — do not cite 2PC as though it describes our exact mechanism.
- WAL does **not** close our gap; Section 4 states this explicitly.
  Do not cite WAL as a solution.
- FIND-027's "linearization point" language is already correctly scoped
  by FIND-027 itself; this note does not add a new formal linearizability
  claim, only confirms the existing wording matches its source concept.
- Helland's and Lamport's papers were read for framing only — neither
  was empirically tested against our candidates in this repository.
- Nothing in PATH-047 through PATH-051 is a distributed transaction
  protocol, a saga engine, or a consensus implementation, and this note
  does not claim otherwise.
- No exactly-once claim is made anywhere, consistent with every prior
  FIND in this series.
- This note makes no claim about real external provider behavior — same
  NOT ESTABLISHED constraint FIND-029 already states.
- "Solves safety" is not "solves the problem" — liveness is tracked as
  a separate, independently important axis throughout Section 4, per
  FIND-029's own safety/liveness discipline.
- No novelty is claimed for any mapping in Section 2 — these are
  standard, decades-old concepts; the contribution (if any) is in the
  precision of which parts fit our tested candidates and which do not.

## 7. Recommended next PATH

**PATH-052 candidate: concurrent recovery of a stuck, in-doubt
`Reserved(K)` (research question E, option 1).**

Not external-contract formalization (options 2/3): Helland's own
argument is that a stronger external atomicity or reconciliation
contract is generally not something we can construct locally, and
FIND-029 already declined to build one for exactly that reason
(reintroducing the atomicity under study, or requiring machinery out of
scope). The concurrent-recovery question, by contrast, is answerable
with the same disposable, local, test-only machinery this whole series
has used, and FIND-029 already flags it by name as the most likely next
step.

**H23 (proposed)**: When completion memory is absent and `Reserved(K)`
already exists (an in-doubt operation, P1-shaped), two or more
concurrent recovery attempts on the *same* canonical occurrence, absent
a single-winner mechanism scoped to the *recovery decision itself* (as
distinct from FIND-027's claim-insertion mechanism), can independently
decide to execute the real effect — producing more than one real effect
for one occurrence. A recovery-scoped single-winner mechanism is
necessary to prevent this, mirroring FIND-027's result at the completion
boundary rather than the claim boundary.

**Why this is the highest-value unanswered question now**: FIND-029's
own probe already showed the failure under a forced, non-scheduled
race; what's untested is whether FIND-027's *existing* claim-boundary
lock, reused unmodified around the recovery decision, already suffices,
or whether a genuinely new lock scoped to recovery (not claim) is
required. That is a precise, falsifiable, small-scope question that
extends the current protocol boundary without inventing new external
contracts.

**Minimal adversarial cases**:
1. Baseline, real threads (not forced sequencing): `Reserved(K)` exists,
   completion memory absent, N concurrent recovery workers race with no
   exclusion — expect duplicate `real_effect_count` under genuine
   scheduling, not just the deterministic ordering FIND-029 used.
2. Same setup, with a lock scoped to the recovery decision for a single
   key — expect exactly one `execute_real_effect` call across N racing
   workers.
3. Two keys, one lock (or per-key locks), workers racing on *different*
   keys concurrently with case 2's lock in place — expect no cross-key
   blocking or starvation (the converse property FIND-027 already
   required of its claim-boundary lock, now required of the recovery
   lock too).
4. Reuse FIND-027's *existing* claim-boundary `Mutex`, unmodified, around
   the recovery decision — does it already prevent case 1's duplicate,
   or does it not apply because the claim step (`Reserved(K)` insertion)
   already completed before recovery began?

**What would falsify H23**: FIND-027's existing claim-boundary lock,
reused unmodified around the recovery decision, already prevents
duplication in case 4 — showing no *new* mechanism is necessary, only
reuse of what FIND-027 already established.

**What would confirm H23**: case 1 reliably duplicates under genuine
concurrent scheduling (extending FIND-029's forced-ordering result to a
real race), case 2 reliably prevents duplication with a mechanism scoped
specifically to the recovery decision, and case 3 shows that mechanism
does not over-serialize across unrelated keys.

## 8. Sources

- Jim Gray, Andreas Reuter. *Transaction Processing: Concepts and
  Techniques.* Morgan Kaufmann, 1993. (Atomicity, WAL, recovery, commit
  protocols, in-doubt transactions — general chapters on logging and
  recovery, and on distributed transactions/2PC.)
- Philip A. Bernstein, Vassos Hadzilacos, Nathan Goodman. *Concurrency
  Control and Recovery in Database Systems.* Addison-Wesley, 1987.
  (Serializability; recovery under concurrent access.)
- Martin Kleppmann. *Designing Data-Intensive Applications.* O'Reilly,
  2017. Chapter 8 ("The Trouble with Distributed Systems" — leases,
  fencing tokens) and Chapter 9 ("Consistency and Consensus" —
  linearizability, idempotency/retries).
- Pat Helland. "Life Beyond Distributed Transactions: An Apostate's
  Opinion." CIDR 2007.
- Pat Helland. "Immutability Changes Everything." CIDR 2015.
- Leslie Lamport. "Time, Clocks, and the Ordering of Events in a
  Distributed System." *Communications of the ACM* 21(7), 1978.
- Jim Gray, Leslie Lamport. "Consensus on Transaction Commit." *ACM
  Transactions on Database Systems* 31(1), 2006 (also MSR-TR-2003-96,
  2004). Read for boundary-setting only.
