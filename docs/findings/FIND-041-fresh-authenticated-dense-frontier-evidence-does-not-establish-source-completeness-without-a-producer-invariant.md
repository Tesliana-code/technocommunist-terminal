# FIND-041 — Fresh, Authenticated, Dense Frontier Evidence Does Not Establish Source Completeness Without a Producer Invariant

Status: OBSERVED / DERIVED / NOT ESTABLISHED
Protocol: PP-0.1-draft
Evidence: PATH-063 (`tests/pathological_cases.rs`, `path_063_*`)
Canonical prior commit: `0c735ed`

## Hypothesis under test

H34 (predeclared, unchanged):

> Authenticated source metadata such as a claimed high-water mark, a fresh response, contiguous records, or dense "no change" coverage cannot by itself establish that the source is semantically complete through U if a stale source can emit the same candidate-visible evidence. A source-frontier mechanism becomes sufficient only under an explicit producer invariant tying "frontier >= U" to actual observation of every authoritative event through U. That producer invariant is not established merely by serializing, authenticating, or freshly returning the frontier value.

The experiment was written to disprove H34. Candidate E combines every visible mechanism tested.

## Prior boundary

FIND-040 (PATH-062): a trusted pinned root authenticates where a witness came from, but not whether it is interval-complete. Two authenticated issuers can agree and still share a stale source. A completeness grant through U makes the rule conditionally sound, but the mechanism does not derive the grant. The remaining load-bearing assumption was that the authenticated issuer's source is complete through U.

## Test model

- Reuses the PATH-061 world, observation, verdict and soundness check. Also reuses the PATH-062 test-local mix, signing key and verifier id, together with its idealized-unforgeability assumption: a tag under key k is produced only by the holder of k. None of this is cryptography, networking, storage or a real clock. O = 100, U = 110.
- **Oracle-only**:
  - `Path061World`.
  - `Path063Source`: how far the source has observed (`view_through`) and which authoritative sequences never entered it (`dropped`).
  - `Path063FrontierReport`: how the issuer derives its frontier. `SourceView` reports the source's own position, capped at the answer boundary. `AnswerBoundary` reports a liveness position.
  - The predicates `path_063_oracle_source_complete_through`, `path_063_oracle_producer_invariant_holds` and `path_063_oracle_slot_is_true`.
- **Candidate-visible**: a single authenticated `Path063Response` containing:
  - the issuer verifier id;
  - an optional challenge id;
  - `from`;
  - the record-id anchor;
  - `frontier`;
  - records with ids;
  - dense slots (`Transition(epoch)` or `NoChange`);
  - a tag.

  The candidate also receives the pinned verifier, its own challenge id (77, supplied at or after U) and U.
- The issuer answers from its source. It emits records and dense slots through min(frontier, U) and tags the whole response.
- **Type check**: `frontier` is a plain `u64`. No type or constructor restricts it. Only the unforgeability assumption restricts who can tag it. No type in the model encodes "proven complete".
- One evaluator, `path_063_evaluate`, runs every candidate. It always checks the pin and tag, that `from == O`, that `frontier >= U`, and that records are ordered and in range. Each candidate enables further checks.

Properties kept separate: issuer authenticity, response freshness, source frontier, structural coverage, semantic completeness and use-time latestness.

## Candidates tested

| Candidate | Additional visible checks | Tests |
|---|---|---|
| A — authenticated high-water mark | pin + tag + `frontier >= U` | `path_063_a`, `path_063_b` |
| B — contiguous record ids | A + record ids are `anchor+1, anchor+2, …` with no gap | `path_063_c` |
| C — dense slots | A + one slot for every sequence in (O, U], matching the records | `path_063_d` |
| D — post-U challenge | A + the response is bound to challenge 77 | `path_063_e` |
| E — combined | A + B + C + D | `path_063_f` |
| F — producer invariant | E's visible checks, evaluated in worlds where the oracle invariant holds and then where it is violated | `path_063_g`, `path_063_h` |

## OBSERVED

1. **A control.** A current source authenticates a frontier of U. The result is `CurrentLatest(4)`, which is correct. A missing pin gives `Unresolved` even though the source is complete, so Unresolved here means completeness is unavailable, not that the source is incomplete. A frontier of U−1 gives `Unresolved`. A frozen source that reports its own position claims frontier O and gets `Unresolved` (`path_063_a`).
2. **A fails.** In world Q the source is current and nothing changes. In world L the same trusted issuer and key read a source frozen at O, report the boundary at which they answer, and epoch 4 occurs at 105. The authenticated responses are identical, both with frontier = U. The producer invariant holds in Q and is violated in L. The verdicts are identical, and L is unsound (`path_063_b`).
3. **B fails.** In world K the only change is epoch 4 at 102. World D also has epoch 5 at 105, and that event never entered the source. The source honestly reports its own view. Both responses show record 501 with no visible gap, and they are identical. B returns `CurrentLatest(4)` in both, which is wrong in D (the truth is 5). A real visible id gap, tagged by the issuer, is rejected (`path_063_c`).
4. **C fails.** Both Q and L carry all 10 slots for (100, 110]. Every slot in Q is true. In L, slot 105 says `NoChange` while the world has a transition at 105. The responses are identical, and L is unsound. A missing slot is rejected (`path_063_d`).
5. **D adds replay resistance only.** A response bound to an earlier challenge (12) is rejected, even though its tag is genuine. Rebinding that response to challenge 77 breaks the tag. A response with no challenge is rejected. A live issuer reading a source frozen at O still answers challenge 77 correctly. It reports `CurrentLatest(3)`, which is unsound (`path_063_e`).
6. **E fails, and this is the central result.** The following are all fixed across worlds Q and L: the trusted root, the issuer key, the frontier policy (honest `SourceView`), the source `view_through`, and challenge 77. Only `dropped` differs: empty in Q, [105] in L. Both responses pass every visible check: tag, challenge binding, frontier ≥ U, contiguous records, dense slots, and slot/record agreement. The visible input is identical, and the correct verdicts differ (3 vs 4). E returns `CurrentLatest(3)` in both. The frozen-source liveness-frontier issuer produces the byte-identical response in L as well (`path_063_f`).
7. **F is sound under the producer invariant.** The test covers 6 worlds × 7 source/report configurations: current, frozen at O, frozen at 105, lossy at [105] and lossy at [103, 108]. E's verdict is sound in every case where the oracle invariant holds, and correct in every case where E commits. Frozen sources that report their own position satisfy the invariant and get `Unresolved` (`path_063_g`).
8. **F does not derive the invariant.** Across the same 42 cases, whenever E commits, it is sound **exactly** when the hidden invariant holds. At least one committed case violates it. A token from an invariant-holding world is byte-identical to one from an invariant-violating world. The violating issuer reports exactly its source's own view, so no dishonest reporting is involved (`path_063_h`).

## DERIVED

- **An authenticated frontier is not epistemically different from the PATH-062 completeness grant.** Both are statements, not checkable by the candidate, that a source is complete through U. The frontier comes from the same party whose completeness it asserts. Authentication binds who said it, and says nothing about whether it is true (observation 2).
- **Structural coverage is not semantic completeness.** Contiguous ids show that nothing the source numbered is missing. Dense slots show that every slot has a record. Neither shows that every authoritative event reached the source, or that every slot record is true (observations 3–4). Both detect real structural defects: a visible gap or a missing slot.
- **Freshness is not source currentness.** A challenge proves only that the response was generated after the candidate chose the challenge. It rules out replay (observation 5). It does not bind the time the source last observed authoritative history. "Freshly answered" and "fresh underlying source" are different properties.
- **Combining every visible mechanism leaves the gap in place (observation 6).** The strongest pair fixes everything the prompt listed except hidden source completeness, and no function of the visible input can be sound in both worlds while committing.
- **The load-bearing property is a producer invariant: "issued frontier f ⇒ the source holds every authoritative event ≤ f".** Under it, the combined candidate is sound (observation 7). The candidate cannot check the invariant, and nothing in the token encodes it (observation 8).
- **Honest self-reporting enforces the invariant only for gaps the source knows about.** A frozen source that reports its own position satisfies the invariant. A source that silently lost an event, reporting the same way, violates it without any dishonesty (observations 1, 7 and 8). Enforcing the invariant would require the producer to know about events it never observed.

H34 outcome: **survived** every attempt to disprove it, including the combined candidate. Its conditional clause is confirmed: a positive result appears only under the producer invariant, and the invariant is not derived.

## HYPOTHESIZED

- The producer invariant restates the completeness problem at the boundary between the source and the authoritative world. Discharging it may need the authority to be structurally unable to emit an event without it entering the source, for example if the source is itself the authority's only write path. Not tested.
- Analogies to log offsets, replication positions or heartbeats are noted only as analogies. None are modeled.

## NOT ESTABLISHED

- Any property of real database replication, Kafka offsets, WAL/LSN, vector clocks, consensus, quorum, linearizability, digital signatures, PKI, transparency logs, hardware roots or wall-clock freshness.
- Collision resistance or unforgeability of the test primitives. They are trivially forgeable, and unforgeability is an assumption the model's parties obey.
- How any producer could enforce the invariant for losses it does not know about.
- That frontier, contiguity, dense coverage or challenges are useless. Each detects a specific defect: an uncovered frontier, a visible gap, a missing slot or a replay.
- Any production type or wiring. `src/` is untouched.
- Novelty.

## Strongest counterexample

`path_063_f`: the candidate-visible input is identical, and every visible mechanism passes:

```
observation = (epoch 3 @ O=100)
pinned      = A11CE   (genuinely the authority's verifier in both worlds)
challenge   = 77      (supplied at/after U; bound in both responses)
response    = { issuer: A11CE, challenge: 77, from: 100, anchor: 500,
                frontier: 110, records: [],
                slots: 101..=110 all NoChange, tag: valid under A11CE }
use         = U=110
```

- Oracle world Q: no transition after 10. The source is `{ view: through U, dropped: [] }`. The invariant holds. The latest at U is 3.
- Oracle world L: epoch 4 at 105. The source is `{ view: through U, dropped: [105] }`. The issuer is the same, with the same key, the same honest `SourceView` report and the same challenge. The invariant is violated. The latest at U is 4.

Candidate E returns `CurrentLatest(3)` in both worlds. The only difference between them is hidden source completeness.

## Outcome classification

- A (authenticated frontier): **negative for completeness**. It is equivalent to a completeness grant issued by the source itself.
- B (contiguous ids): **positive for visible structural gaps**, **negative for semantic completeness**.
- C (dense slots): **positive for missing slots**, **negative for slot truth**.
- D (challenge): **positive for replay resistance**, **negative for source currentness**.
- E (combined): **negative**, disproved by the identical-input pair.
- F: **CONDITIONAL positive**. Sound under the producer invariant, which the candidate does not derive and the token does not carry.

## Stop condition / next boundary

Use-time latestness in the model now rests on: a trusted pinned root, idealized unforgeability, and the producer invariant "issued frontier f ⇒ the source holds every authoritative event ≤ f". Visible frontier, coverage and freshness evidence cannot supply the last of these. The next boundary is whether the invariant can be discharged structurally, by making the source the only path through which an authoritative event can exist, or whether it must remain an explicit assumption. That belongs to a separate PATH.

## Validation state

- `cargo test --test pathological_cases path_063`: 8 passed, 0 failed
- `cargo test --test pathological_cases path_062`: 12 passed, 0 failed (regression)
- `cargo check --bins`: clean
- `git diff --check`: clean
- rustfmt: not applied (`--check` reports pre-existing whole-file drift; no whole-file formatting)
- production `src/`: untouched
- PostgreSQL / ignored tests: not run
- no commit, no push
