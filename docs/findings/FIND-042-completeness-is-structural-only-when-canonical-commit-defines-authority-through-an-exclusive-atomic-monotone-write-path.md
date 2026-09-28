# FIND-042 — Completeness Is Structural Only When Canonical Commit Defines Authority Through an Exclusive, Atomic, Monotone Write Path

Status: OBSERVED / DERIVED / NOT ESTABLISHED
Protocol: PP-0.1-draft
Evidence: PATH-064 (`tests/pathological_cases.rs`, `path_064_*`)
Canonical prior commit: `ccbc786`

## Hypothesis under test

H35 (predeclared, unchanged):

> If the model defines authoritative existence such that an event becomes authoritative ONLY by successfully entering a single canonical source log, then source completeness through its own committed frontier is structural rather than observational. Under that model, "frontier >= U" may become sufficient for latestness without a separate completeness grant. However, this positive result is conditional on the exclusivity and atomicity of the authoritative write path. If authoritative state may be created before append, beside append, or through any bypass/fork, the old completeness counterexample returns.

The experiment was written to disprove H35. It includes a search for a weaker structural rule that would be enough.

## Prior boundary

FIND-041 (PATH-063): authenticated frontier claims, contiguous record ids, dense NoChange slots and fresh challenges, alone or combined, do not establish source completeness. The load-bearing producer invariant was: issued frontier f ⇒ the source contains every authoritative event ≤ f. The stop condition asked whether making the source the only path through which authority exists would discharge that invariant structurally.

## Test model

- `Path064System` is a transition system parameterized by `Path064Architecture`. `apply(op)` **rejects** any operation the architecture does not have. "Unconstructible under this model" means that no accepted operation sequence reaches the state.
- The vocabulary is kept distinct:
  - **Proposed**: an entry in `proposals`. A proposal is never authoritative by itself.
  - **Committed**: a `Transition` entry in the canonical log.
  - **Authoritative**: `path_064_oracle_authoritative`, the single, oracle-only definition of authority, which differs per architecture.
  - **Frontier**: the highest committed canonical sequence, counting both transition and checkpoint entries.
- **Candidate-visible**: `Path064Visible`, which is the canonical log entries plus the frontier, and nothing else. The candidate `path_064_candidate_latest` returns `Unresolved` if frontier < U. Otherwise it returns the latest committed transition at or before U.
- **Oracle-only**: the proposals and the world, promotion, bypass, fork and acknowledgement sets. The architecture's authority definition decides which of these count.
- Operations: `Propose`, `DeclareWorld`, `Append` (accepted only if the sequence can be placed), `FailAppend`, `Abort`, `Promote`, `Ack`, `Checkpoint`, `Bypass`, `ForkCommit`.
- **Bounded exhaustive search**: `path_064_explore` enumerates every accepted sequence up to depth 6 over a fixed 15-operation alphabet. The alphabet covers proposals at 105, 110 (= U) and 115, append, fail, abort, promote, ack, checkpoints at 110 and 120, bypass and fork. Reachable states explored per architecture: A 93; B 6,798; C/F 6,283; F′ 11,992; D 27,139; E 27,139; G1 11,621; G2 6,798.
- Genesis: epoch 3 at sequence 10, committed and authoritative in every architecture. U = 110. No storage, network, consensus or clock is modeled.

## Candidates tested

| Candidate | Architecture | When an event becomes authoritative | Tests |
|---|---|---|---|
| A | `AuthorityBeforeAppend` | at `DeclareWorld`, before and independently of the append | `path_064_a` |
| B | `AppendThenPromote` | at `Promote`, which needs a prior successful append | `path_064_b` |
| C | `AtomicCommit` | at `Append`: commit and authority are one step | `path_064_c` |
| D | `AtomicCommitWithBypass` | at `Append`, **or** at `Bypass` without an append | `path_064_d` |
| E | `TwoAuthorityLogs` | at an `Append` to log A, **or** at `ForkCommit` to log B | `path_064_e` |
| F | `AtomicCommit`, used as the positive control | at `Append` only; the log is monotone | `path_064_f`, `path_064_g` |
| F′ | `AtomicCommitWithBackfill` | at `Append`, but appends below the frontier are accepted | `path_064_g` |
| G1 | `PreCommitAck` | at `Append`, **or** when consumers receive an ACK, which may precede the commit | `path_064_h` |
| G2 | `PostCommitAck` | at `Append`; an ACK is issued only for committed entries | `path_064_h` |

## OBSERVED

1. **A: the hidden-loss counterexample returns.** Epoch 4 becomes authoritative at 105, its append fails, and a checkpoint takes the frontier to 110. The canonical log and frontier are identical to the quiet world's, and the candidate returns `CurrentLatest(3)` in both. That is unsound in the loss world, where the latest is 4. A late mirror append at 105 is refused, because 105 is below the frontier (`path_064_a`).
2. **B closes one direction of the iff and opens the other.** A failed append or an abort creates no authority. Promotion without a commit is rejected. Retry after a failure works, and a duplicate proposal id is rejected. The exhaustive search finds **no** state where an authoritative event is uncommitted, but it does find states where a committed entry is not authoritative. Pair: the same log, with epoch 4 committed at 105 and a checkpoint at 110, promoted in one world and never promoted in the other (crash between append and promotion). The visible input is identical. The candidate returns `CurrentLatest(4)` in both, which is unsound in the unpromoted world, where the latest is 3 (`path_064_b`).
3. **C: hidden authority is unconstructible under this model.** Across 6,283 reachable states, neither direction of the iff fails and the candidate never commits an unsound verdict. Each negative construction from A, D, E and G1 is rejected at the first operation the architecture lacks: `DeclareWorld`, `Bypass`, `ForkCommit`, or an `Ack` before commit (`path_064_c`).
4. **D: a bypass restores the counterexample.** `Bypass` puts epoch 4 at 105 and the log is checkpointed to 110. The log and frontier are identical to the quiet world's, and the candidate is unsound in the bypass world. Every committed entry is still authoritative, so only the "authoritative ⇒ committed" direction fails (`path_064_d`).
5. **E: a complete log does not cover a second authority log.** `ForkCommit` puts epoch 4 at 105 into log B. Log A is internally complete, since every entry it committed is authoritative. Its visible evidence is identical to the quiet world's, and the candidate is unsound (`path_064_e`).
6. **F: the positive control holds in every tested world:**
   - no change;
   - one change;
   - multiple changes;
   - a failed proposal before U;
   - a failure followed by a successful retry;
   - duplicate content, with the second append refused;
   - a change exactly at U;
   - a proposal after U;
   - an aborted transition;
   - a frontier short of U, which gives `Unresolved`.

   In each one, the iff holds and the verdict matches the oracle. The refusals are also checked: a duplicate id, a second append at the same sequence, an append after an abort, and an append below the frontier (`path_064_f`).
7. **Monotone sequencing is also load-bearing (F vs F′).**
   - **F:** from every reachable state with frontier ≥ U, no accepted operation changes the authoritative latest at U. This is checked one step at a time, exhaustively (`path_064_g`).
   - **F′:** the log is atomic and exclusive, but it accepts appends below its frontier. A verdict of `CurrentLatest(3)`, sound when issued at frontier 110, becomes unsound after an append at 105 is accepted. The iff still holds at every instant in F′: 11,992 states with no violation in either direction.
8. **G1: a pre-commit ACK recreates an authoritative-but-unlogged state** when consumers treat the ACK as authority. Propose, ACK, then lose the proposal (abort, standing in for a crash). The log and frontier are identical to the quiet world's, and the candidate is unsound. **G2:** a pre-commit ACK is not an available operation. Across 6,798 states there is no violation and no unsound verdict (`path_064_h`).
9. **The iff matrix (`path_064_i`), from exhaustive search at depth 6.** The candidate is unsound somewhere exactly when at least one direction fails.

| Architecture | authoritative ⇒ committed | committed ⇒ authoritative | candidate unsound somewhere |
|---|---|---|---|
| A before-append | fails | holds | yes |
| B append-then-promote | holds | fails | yes |
| C/F atomic commit | holds | holds | no |
| F′ atomic + backfill | holds | holds | no (at each instant) |
| D bypass | fails | holds | yes |
| E two logs | fails | holds | yes |
| G1 pre-commit ACK | fails | holds | yes |
| G2 post-commit ACK | holds | holds | no |

## DERIVED

- **Canonical append discharges the producer invariant, but only by changing what authority is.** In A, authority exists independently and the source observes it: the PATH-063 architecture. The loss counterexample returns there unchanged. In C/F, the canonical commit is what *makes* an event authoritative. Completeness is then **structural relative to the authority definition**. It holds because no accepted transition can produce an authoritative event outside the log, not because the source observed an external world completely. This is a different authority architecture, not better observation.
- **Each property removes a specific counterexample:**
  - **Authority must be defined by the canonical commit.** In A it isn't, and the result is hidden loss.
  - **Exclusivity:** no other operation may create authority. D, E and G1 each break "authoritative ⇒ committed" by adding a single extra authority-creating path. In every one of them, the canonical log itself stays internally complete.
  - **Atomicity:** commit and authority must not be separate steps. B separates them and breaks "committed ⇒ authoritative", so the log over-reports authority.
  - **Monotone sequencing:** no commit below the frontier. F′ keeps the iff at every instant, but the frontier stops closing history: "frontier ≥ U" no longer means latest-at-U is final.
- **Frontier ≥ U is sufficient for latestness only when all four hold**, as in F. There, every authoritative event ≤ U corresponds to a committed entry ≤ frontier by construction, and no later accepted operation can change latest-at-U.
- **The ACK result is exclusivity seen from outside.** If any signal other than the canonical commit is treated as authority by consumers, that signal is a second write path.
- **No weaker structural rule was found.** Dropping any one of the four properties produced a reachable unsound verdict, either at an instant or after a later accepted operation.

H35 outcome: **survived, and sharpened.** The positive result holds under an exclusive, atomic single write path, as predeclared. Two further conditions turned out to be load-bearing as well. The first is that authority must be *defined* by the canonical commit (the ontology change). The second, which H35 did not name, is that the log's sequencing must be monotone.

## HYPOTHESIZED

- The remaining boundary may no longer be about evidence. It may be about enforcement: whether any real component can guarantee that no other path creates something consumers treat as authoritative. That is a question about system boundaries and consumer discipline, which this model does not cover.
- Write-ahead logs, single-leader sequencers and commit acknowledgement are noted only as analogies. None are modeled.

## NOT ESTABLISHED

- Any property of real databases, WAL, Kafka, Raft/Paxos, consensus, linearizability, distributed transactions, exactly-once delivery, two-phase commit, replication or durability hardware.
- That exclusivity, atomicity or monotone sequencing can be enforced in practice, or detected from inside the log. A bypass, fork or ACK leak leaves the canonical log byte-identical, so the log cannot detect it.
- Unboundedness. The search is exhaustive only up to depth 6 over a fixed alphabet. The unconstructibility claim for C/F also rests on its definitional structure, not only on the search: authority is defined as the committed transitions, and the only authority-creating operation is `Append`.
- That an external world is complete. F makes no claim about events outside its authority definition.
- Any production type or wiring. `src/` and the SUBNOR operator UI are untouched.
- Novelty.

## Strongest counterexample

**Negative (`path_064_d`, bypass).** Everything the positive model relies on is present: an atomic canonical commit, a monotone log, and every committed entry authoritative. Only exclusivity is violated:

```
visible (both worlds) = { entries: [T(10, epoch 3), Checkpoint(110)], frontier: 110 }
use                   = U=110
```

- World Q: no other operation. The authoritative latest at U is 3.
- World L: `Bypass { 105, epoch 4 }`. The authoritative latest at U is 4.

The candidate returns `CurrentLatest(3)` in both. A (lost mirror), E (fork) and G1 (pre-commit ACK) produce the same visible input with the same verdict split.

**Positive (F).** The corresponding negative world would need an authoritative event with no committed entry. In `AtomicCommit`, authority is defined as the committed transitions, and the only operation that adds one is `Append`. `DeclareWorld`, `Bypass`, `ForkCommit` and a pre-commit `Ack` are not operations of this architecture: each is rejected at the point of use (observation 3). Because `Append` is also the commit, B's converse state (committed but not authoritative) has no transition that produces it either. Because appends below the frontier are rejected, no later operation can change latest-at-U once frontier ≥ U (observation 7). The negative world is therefore unconstructible under this model, not merely undetected.

## Outcome classification

- A: **negative**. The hidden-loss counterexample returns.
- B: **partially positive**. There is no hidden authority, but it over-reports during the gap between commit and promotion.
- C/F: **CONDITIONAL positive, structural**. Completeness holds relative to the authority definition, given exclusivity, atomicity and monotone sequencing.
- D, E, G1: **negative**. Each breaks exclusivity and returns the counterexample, with identical visible input.
- F′: **negative for frontier closure**. The iff holds at every instant, but a verdict can be invalidated by a later accepted operation.
- G2: **positive**. A post-commit ACK does not leak authority.

## Stop condition / next boundary

Within the model, the producer invariant has been turned from an observational assumption into a structural one. It holds by construction when canonical commit *is* authority, through one exclusive, atomic, monotone write path. The trust boundary is now the **enforcement of that architecture**: that no other path, fork or pre-commit signal is treated as authoritative by anyone, and that the log never accepts commits below its frontier. The canonical log cannot observe a violation of either. The next boundary is whether that enforcement can itself be made candidate-visible, or whether it is irreducibly a property of the system's boundary. That belongs to a separate PATH.

## Validation state

- `cargo test --test pathological_cases path_064`: 9 passed, 0 failed
- `cargo test --test pathological_cases path_063`: 8 passed, 0 failed (regression)
- `cargo check --bins`: clean
- `git diff --check`: clean
- rustfmt: not applied (no whole-file formatting of `tests/pathological_cases.rs`)
- production `src/`: untouched; SUBNOR operator UI untouched
- PostgreSQL / ignored tests: not run
- no commit, no push
