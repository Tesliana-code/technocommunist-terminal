# FIND-037 — Survival and Replica Agreement Do Not Establish Authority-History Completeness

Status: OBSERVED / DERIVED / NOT ESTABLISHED
Protocol: PP-0.1-draft
Evidence: PATH-059
Canonical prior commit: PATH-058 / FIND-036 at `104fa02`

## Hypothesis under test

FIND-036 relocated authority-evidence authenticity to an external authority record.

PATH-059 asks the next recovery question:

Does recovery of a surviving authority record prove that the recovered authority history is complete and that no later authoritative transition existed and was lost?

The tested answer is no.

The experiment then strengthens the candidate from one recovered copy to two agreeing recovered copies.

## Test model

PATH-059 uses a test-local authority record containing:

- epoch
- lineage
- transition identity
- parent transition identity

`Path059RecoveryVerdict` has two states:

- `RecoveredLatest(record)`
- `Unresolved`

The ground-truth helper knows the actual latest authority record.

Recovery candidates do not.

This distinction exists only as a test oracle.

## OBSERVED

### 1. Complete recovery control

PATH-059-A recovers epoch 3 in a world where epoch 3 is actually the latest transition.

The naive recovery candidate returns:

`RecoveredLatest(epoch 3)`

The ground-truth oracle agrees.

### 2. A surviving stale record can be misclassified as latest

PATH-059-B recovers epoch 3 while the modeled world says epoch 4 existed later and its recovered evidence was lost.

The naive recovery candidate still returns:

`RecoveredLatest(epoch 3)`

The ground-truth oracle returns:

`Unresolved`

Therefore survival of epoch 3 does not prove that epoch 3 was the final authoritative transition.

### 3. Complete and lossy histories can have identical recovered state

PATH-059-C constructs two worlds.

World C:

- epoch 3 survived
- epoch 3 is actually latest

World L:

- epoch 3 survived
- epoch 4 existed later
- epoch 4 was lost from recovery-visible state

Both worlds expose the same recovered value:

`Some(epoch 3)`

The recovery candidate therefore produces the same result in both worlds.

The correct modeled recovery verdict differs.

This establishes the tested identifiability boundary.

### 4. No surviving authority record remains unresolved

PATH-059-D confirms the conservative control.

Missing recovered authority state yields:

`Unresolved`

The test does not interpret absence as proof that no authority history existed.

### 5. Agreement between recovered replicas works in the complete control

PATH-059-E gives two recovered copies containing the same epoch-3 record in a world where epoch 3 is actually latest.

The replica-agreement candidate returns:

`RecoveredLatest(epoch 3)`

This is the positive control for replica agreement.

### 6. Two agreeing replicas can both be stale

PATH-059-F gives both recovered copies the same epoch-3 record while the modeled world contains a later epoch-4 transition that is absent from both recovered copies.

The candidate sees agreement and returns:

`RecoveredLatest(epoch 3)`

The ground-truth recovery verdict remains:

`Unresolved`

Therefore agreement between the tested replicas does not establish historical completeness.

### 7. Complete and correlated-loss worlds can expose identical replica views

PATH-059-G constructs two replica views:

complete world:

`(epoch 3, epoch 3)`

correlated-loss world:

`(epoch 3, epoch 3)`

The views are identical.

Replica agreement alone contains no observable fact proving that the shared record is historically complete.

### 8. Identical replica views can require different correct recovery verdicts

PATH-059-H closes the identifiability argument explicitly.

The complete world and correlated-loss world expose identical replica views and therefore produce identical observed candidate results.

The modeled correct recovery verdicts differ:

- complete world -> `RecoveredLatest(epoch 3)`
- correlated-loss world -> `Unresolved`

Therefore a decision function restricted to the tested replica contents cannot distinguish these worlds.

Additional information is required, or recovery must remain unresolved.

## DERIVED

H30 is supported for the tested candidates:

> Recovery of a surviving authority record establishes that the record survived. It does not, from its contents alone, establish that the recovered authority history is complete.

And:

> Agreement between recovered replicas establishes agreement between those recovered replicas. It does not, by itself, establish that no later authoritative transition was lost from all of them.

The critical distinction is:

- positive evidence that a record survived
- negative claim that no later record ever existed

The first is observable in the tested recovery state.

The second is not.

## Relation to FIND-026

FIND-026 established that historical completeness is required before negative lookup evidence can safely imply novelty.

PATH-059 exposes the same epistemic shape at the authority-history recovery boundary.

The application is different:

- FIND-026 concerns replay and occurrence history
- FIND-037 concerns recovered authority-transition history

The shared principle is:

> surviving positive state does not automatically prove completeness of the history around it.

PATH-059 does not claim this principle is novel.

## Safety interpretation

A conservative recovery system can safely use surviving records as evidence that those records exist.

It cannot infer from the tested state alone that:

- no later transition existed
- no later transition was lost
- replica agreement implies complete authority history

If completeness cannot be established by additional evidence, the tested safe result is `Unresolved`.

## NOT ESTABLISHED

PATH-059 does not establish:

- that replication is ineffective
- that two replicas are universally insufficient
- that any particular replica count is sufficient or insufficient
- quorum requirements
- consensus requirements
- append-only log requirements
- cryptographic requirements
- Byzantine fault tolerance
- physical durability guarantees
- independent-failure guarantees
- real storage-device behavior
- real distributed database behavior
- a production recovery architecture
- exactly-once execution
- resolution of FIND-029 historical-effect ambiguity
- novelty

The correlated-loss world is a test construction.

No universal claim is made about real replicated systems.

## Candidate central result

In the tested authority-recovery model, a surviving authority record does not prove that the recovered authority history is complete.

Two agreeing recovered copies also do not prove completeness when a later transition can be absent from both recovered views.

The complete and correlated-loss worlds can expose identical recovery-visible state while requiring different correct recovery verdicts.

Therefore:

> survival proves survival, not completeness.

And:

> replica agreement proves agreement, not historical completeness.

These statements are scoped to the tested PATH-059 candidates.

## Stop condition

PATH-059 reaches the next boundary:

Recovery requires some evidence capable of establishing history completeness, rather than merely exposing surviving records.

The experiment does not establish what form that evidence must take, how it becomes durable, or how its own correctness is recovered after failure.

Those questions belong to a separate PATH.

## Validation state before final sealing

- targeted PATH-059 tests: 8 passed
- targeted failures: 0
- PATH-059 is isolated in `tests/path_059_authority_recovery.rs`
- production `src/`: untouched
- full-suite validation: library 47/47 passed; NCR simulation 2/2 passed; PATH-059 8/8 passed; pathological_cases 477 passed / 6 known unrelated failures (PATH-015/016/017/019/023/026); no new PATH-059 failure
- no commit
- no push
