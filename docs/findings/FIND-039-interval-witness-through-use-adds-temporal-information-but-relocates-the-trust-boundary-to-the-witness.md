# FIND-039 — An Interval Witness Through Use Adds Temporal Information but Relocates the Trust Boundary to the Witness

Status: OBSERVED / DERIVED / NOT ESTABLISHED
Protocol: PP-0.1-draft
Evidence: PATH-061 (`tests/pathological_cases.rs`, `path_061_*`)
Canonical prior commit: `8669f37`

## Hypothesis under test

H32 (predeclared, unchanged):

> A mechanism can establish use-time latestness in the modeled system only if it carries information covering authoritative change through the decision-use boundary. A point observation before use, a "recent" observation, and a repeated read of the same visible stale state are insufficient. A complete change-interval witness through the use boundary may be sufficient, but only under an explicit assumption that the witness itself is authentic and complete. If forged and genuine witnesses can expose identical visible inputs while requiring different verdicts, the trust boundary has been relocated rather than eliminated.

## Prior boundary

FIND-038 (PATH-060): bounded completeness and truthful point-in-time currentness do not establish use-time latestness. Stop condition: use-time latestness requires information about authoritative change between the observation boundary O and the decision-use boundary U.

## Test model

- Boundaries are integer event-sequence values (`Path061ObservationBoundary`, `Path061UseBoundary`). No wall-clock semantics.
- A transition at sequence `s` is in effect at boundary `b` iff `s <= b`.
- **Oracle-only:** `Path061World` (the full transition list), `Path061ReadSource` (current vs frozen), `Path061WitnessReality` (genuine vs forged).
- **Candidate-visible:** `Path061CurrentnessObservation` (epoch N at O), `Path061VisibleRead` (epoch at U), `Path061IntervalWitness` (`from`, `complete_through`, listed transitions). The witness carries no authenticity information.
- Verdicts: `CurrentLatest(epoch)` or `Unresolved`. `Unresolved` is always sound; `CurrentLatest(e)` is sound iff `e` equals the oracle's latest epoch at U.

Properties kept separate throughout: authenticity, historical completeness, interval completeness over (O, U], observation-time currentness, use-time latestness.

## Candidates tested

| Candidate | Rule | Tests |
|---|---|---|
| A — recent enough | `CurrentLatest(N)` if `U - O <= max_distance` | `path_061_a` |
| B — equal re-read | `CurrentLatest(N)` if a read at U returns N | `path_061_b` |
| C — interval witness | Needs witness with `from == O`, `complete_through >= U`, all listed transitions in range; returns latest listed transition `<= U`, else N | `path_061_c`–`f` |
| D — forged witness | Candidate C given a forged witness with a valid shape | `path_061_g`, `path_061_h` |

## OBSERVED

1. **A fails.** O=100, U=102. World A has no transition after O. World B has epoch 4 at 101. The candidate-visible observation `(3 @ 100, U=102)` is identical and truthful in both worlds. The candidate returns `CurrentLatest(3)` in both. The oracle's latest at U is 3 in A and 4 in B, so the candidate is unsound in B. The same counterexample holds for every threshold 2..=8.
2. **B fails.** O=100, U=110. World A has a current read source and no change. World B has epoch 4 at 105 and a read source frozen at O. The candidate-visible `(observation, reread)` is identical: 3 both times. The candidate returns `CurrentLatest(3)` in both and is unsound in B. The source's currentness is the only difference, and it is visible only to the oracle.
3. **C control holds.** A genuine witness covering (100, 110] with no transitions gives `CurrentLatest(3)`, which matches the oracle.
4. **C exposes intervening change.** Genuine witnesses for the quiet world and the changed world (epoch 4 at 105) differ, even though the observation-time currentness evidence is identical. Given the changed-world witness, the candidate returns `CurrentLatest(4)`, which matches the oracle. A transition exactly at U (sequence 110) is also covered and yields `CurrentLatest(4)`.
5. **A witness ending at U-1 stays Unresolved.** Genuine witnesses through 109 are identical for the quiet world and for a world with epoch 4 at exactly 110, yet the oracle verdicts differ. The candidate returns `Unresolved`, which is sound in both worlds.
6. **Missing witness stays Unresolved.** With no witness, the result is `Unresolved`. A witness starting after O (leaving (100, 101] uncovered) also gives `Unresolved`.
7. **Forged and genuine witnesses are identical to the candidate.** World G has a genuine empty witness through U. World F has epoch 4 at 105 and a forged empty witness. The candidate-visible tuple `(observation, witness, U)` is equal across the two worlds. The candidate returns `CurrentLatest(3)` in both: sound in G, unsound in F.
8. **Soundness tracks witness reality exactly.** Take four (world, witness) pairings: two genuine and two forged in opposite directions, one omitting a real change and one inventing a change that never occurred. The candidate commits to `CurrentLatest` in all four. It is sound exactly when the oracle-only `Path061WitnessReality` is `Genuine`. The invented-change forgery produces `CurrentLatest(4)` where the oracle says 3.

## DERIVED

- In the model, "recent" (a small O→U distance) is not latestness. Distance says nothing about whether a transition occurred in (O, U].
- A matching re-read is not latestness unless the read source has a currentness/completeness property. That property is not in the visible read.
- A genuine interval-complete witness over (O, U] carries information that PATH-060 lacked. Genuine witnesses separate worlds that the PATH-060 inputs could not separate (observation 4). With that information, Candidate C is sound in every genuine case tested.
- Coverage must reach U itself. Stopping one step short leaves a modeled gap that the witness cannot rule out (observation 5).
- The candidate cannot tell a genuine witness from a well-formed forged one, because authenticity is not in its input (observations 7–8). The witness removes the *temporal* gap but not the *trust* gap. Candidate C's correctness depends entirely on an exogenous assumption that the witness is authentic and interval-complete.

H32 outcome: **supported** for all tested candidates, including its conditional clause. The interval witness is sufficient only under the exogenous authenticity/completeness assumption. The trust boundary moves from "is N still latest?" to "is this interval witness genuine and complete?"

## HYPOTHESIZED

- Any mechanism that brings use-time latestness down to candidate-visible evidence will need some property of that evidence, such as authenticity or completeness of the source, that is not itself established by the evidence. This is what happened in FIND-036 and FIND-037, and again here. PATH-061 shows it for one witness shape only.
- Interval-witness authenticity may reduce to the FIND-036 authenticity problem applied to a different object. Not tested.

## NOT ESTABLISHED

- Any claim about real timestamps, clocks, leases, locks, linearizability, consensus, quorum, or cryptographic signatures.
- That a gap of 1 is safe in general. With an integer sequence, `U = O + 1` leaves no room for a transition strictly between O and U, but that comes from the discrete model encoding adjacency. It is not evidence about real systems. Candidate A was not claimed sound for distance 1.
- That interval witnesses can be produced, authenticated, or made complete in practice.
- How to detect forged witnesses. No authenticity mechanism was added, per the no-post-hoc-patch rule.
- Any production type, protocol, or economic-allocation wiring. `src/` is untouched.
- Novelty.

## Strongest counterexample

`path_061_g`: the candidate-visible input is identical in both worlds:

```
observation = (epoch 3 @ O=100)
witness     = { from: 100, complete_through: 110, transitions: [] }
use         = U=110
```

- Oracle world G: no transition after 10. The witness is genuine. The latest at U is 3.
- Oracle world F: epoch 4 at 105. The witness is forged. The latest at U is 4.

Candidate C returns `CurrentLatest(3)` in both. No function of the visible input can be sound in both worlds while committing to a verdict. The only sound verdict that works for both worlds is `Unresolved`, and that throws away the information the witness was supposed to add.

## Outcome classification

- Candidates A and B: **negative**, disproved by indistinguishable worlds.
- Candidate C: **conditionally positive**. It adds genuine temporal information through U and is sound for genuine witnesses. Coverage ending before U, or a missing witness, gives `Unresolved`.
- Candidate D: **negative for self-sufficiency**. The interval witness relocates the truth boundary to witness authenticity and completeness. It does not manufacture truth.

## Stop condition / next boundary

Use-time latestness in the model reduces to the authenticity and interval-completeness of a witness covering (O, U]. The next boundary is what, if anything, can make that witness's authenticity and interval completeness visible to the candidate without another exogenous trust assumption. That belongs to a separate PATH.

## Validation state

- `cargo test --test pathological_cases path_061`: 8 passed, 0 failed
- `cargo check --bins`: clean
- `git diff --check`: clean
- production `src/`: untouched
- PostgreSQL / ignored tests: not run
- no commit, no push
