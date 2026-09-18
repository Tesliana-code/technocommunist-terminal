# FIND-038 — Bounded Completeness and Point-in-Time Currentness Do Not Establish Use-Time Latestness

Status: OBSERVED / DERIVED / NOT ESTABLISHED
Protocol: PP-0.1-draft
Evidence: PATH-060
Canonical prior commit: PATH-059 / FIND-037 at `a55df9e`

## Hypothesis under test

FIND-037 established that surviving authority state and replica agreement do not prove complete authority history.

PATH-060 grants stronger evidence.

PATH-060A grants a truthful bounded witness:

`history complete through epoch N`

PATH-060B additionally grants a truthful point-in-time currentness witness:

`epoch N was current when observed at time T`

The question is whether either witness establishes that N is still the latest authoritative state when a later recovery decision is used.

## Test model

The experiment uses test-local records and witnesses only.

A completeness witness states that history is complete through an epoch.

A currentness witness states which epoch was latest at a modeled observation point.

Neither witness contains information about transitions occurring after its observation boundary.

`Path060LatestVerdict` contains:

- `CurrentLatest(record)`
- `Unresolved`

Ground truth separately identifies the actual latest record at decision-use time.

## OBSERVED

### 1. Bounded completeness succeeds in the control world

PATH-060-A recovers epoch 3 with a truthful witness that history is complete through epoch 3.

No later transition exists.

The candidate and ground-truth oracle both return:

`CurrentLatest(epoch 3)`

### 2. Truthful bounded completeness can coexist with a later transition

PATH-060-B uses the same truthful completeness witness through epoch 3.

Epoch 4 is subsequently present in the modeled world.

The naive candidate still returns:

`CurrentLatest(epoch 3)`

The correct modeled use-time verdict is:

`Unresolved`

Therefore:

> complete through epoch N does not imply that no transition occurred after N.

### 3. Identical bounded-completeness inputs can require different verdicts

PATH-060-C gives the candidate identical observable input in two worlds.

World A:

- recovered epoch 3
- complete through epoch 3
- actual latest remains epoch 3

World B:

- recovered epoch 3
- complete through epoch 3
- epoch 4 exists later

The candidate observes the same record and witness in both worlds.

The correct modeled verdict differs.

Therefore bounded historical completeness does not by itself establish current latestness.

### 4. Missing completeness evidence remains unresolved

PATH-060-D confirms the conservative control.

Without a completeness witness, the tested candidate returns `Unresolved`.

### 5. A truthful point-in-time currentness witness succeeds in the no-change control

PATH-060-E adds a truthful witness saying epoch 3 was current at observation 100.

No later transition occurs.

The candidate and ground-truth oracle agree that epoch 3 is latest.

### 6. A truthful currentness witness can become stale before use

PATH-060-F keeps the same truthful observation:

`epoch 3 was current at observation 100`

Epoch 4 is then created before the modeled recovery decision is used.

The witness remains historically truthful.

The candidate nevertheless misclassifies epoch 3 as current latest at use time.

Thus:

> truth at observation time does not establish truth at later use time when the authoritative state can change in between.

### 7. Identical truthful witnesses can require different use-time verdicts

PATH-060-G closes the identifiability argument.

Two worlds expose identical:

- recovered authority record
- bounded completeness witness
- truthful point-in-time currentness witness

In one world no later transition occurs.

In the other, epoch 4 occurs after the observation.

The observable recovery input is identical.

The correct use-time verdict differs.

A decision function restricted to those witnesses therefore cannot distinguish the worlds.

## DERIVED

H31 is supported for the tested candidates:

> A truthful bounded completeness witness establishes completeness only through its stated historical cut.

It does not establish that the cut remains current.

And:

> A truthful point-in-time currentness witness establishes currentness only at its modeled observation boundary.

It does not establish use-time latestness if authority state can change after observation and before decision use.

The tested distinction is temporal:

- historical completeness
- observation-time currentness
- use-time currentness

These are not interchangeable properties.

## Safety interpretation

The tested candidates may safely interpret a truthful bounded witness as evidence about its bounded cut.

They may safely interpret a truthful currentness witness as evidence about its observation point.

Neither tested witness licenses an unconditional claim that the same state is still latest at a later use boundary.

If later change cannot be excluded or observed, the tested conservative result is `Unresolved`.

## NOT ESTABLISHED

PATH-060 does not establish:

- that timestamps solve or fail to solve the problem
- clock synchronization requirements
- lease requirements
- locking requirements
- linearizability requirements
- serializability requirements
- consensus requirements
- quorum requirements
- cryptographic requirements
- a universal freshness protocol
- a production database architecture
- real network timing behavior
- real storage timing behavior
- exactly-once execution
- resolution of FIND-029 historical-effect ambiguity
- novelty

The observation and use boundaries are test-local modeled events.

No wall-clock semantics are claimed.

## Candidate central result

In the tested authority-recovery model:

> complete through N does not mean nothing happened after N.

And:

> current at observation time T does not mean current at later decision-use time.

Truthful evidence can become insufficient without becoming false.

The missing property is information about change between the evidence boundary and the decision-use boundary.

These statements are scoped to PATH-060.

## Stop condition

PATH-060 reaches the next boundary:

Use-time latestness requires some mechanism that relates authoritative change to the decision-use boundary rather than only to an earlier observation.

The experiment does not establish what mechanism is required.

That belongs to a separate PATH.

## Validation state before final sealing

- targeted PATH-060 tests: 7 passed
- targeted failures: 0
- PATH-060 isolated in `tests/path_060_history_completeness_witness.rs`
- production `src/`: untouched
- full-suite validation: library 47/47 passed; NCR simulation 2/2 passed; PATH-059 8/8 passed; PATH-060 7/7 passed; pathological_cases 477 passed / 6 known unrelated failures (PATH-015/016/017/019/023/026); no new PATH-060 failure
- no commit
- no push
