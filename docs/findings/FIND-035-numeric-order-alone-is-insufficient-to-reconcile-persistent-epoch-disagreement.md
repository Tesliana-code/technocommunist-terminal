# FIND-035 — Numeric Order Alone Is Insufficient to Reconcile Persistent Epoch Disagreement

Status: OBSERVED / DERIVED / NOT ESTABLISHED
Protocol: PP-0.1-draft
Evidence: PATH-057
Canonical prior commit: PATH-056 / FIND-034 at `56aa7d7`

## Hypothesis under test

PATH-056 left System A ownership authority and System B effect authority able to disagree about the current epoch.

PATH-057 tests whether persistent disagreement can be reconciled from epoch numbers alone, or whether repair requires surviving evidence about the authority history that produced those epochs.

The tested models are test-local only.

## Test-local trust model

In PATH-057, trusted evidence means only that the test fixture explicitly marks an epoch or transition as originating from the modeled canonical issuer, System A, inside a modeled lineage.

It does not establish:

- cryptographic authenticity
- signatures or PKI
- tamper resistance
- physical durability
- real-world authentication
- Byzantine safety

The trust marker is an assumption of the model, not an implementation of trust.

## OBSERVED

### 1. Numeric maximum is not authority

PATH-057-A and PATH-057-B provide a naive max-wins control.

With A=2/B=1, numeric max suggests advancing B.

With A=1/B=2, numeric max suggests advancing A.

The later adversarial cases show why this arithmetic rule is insufficient by itself.

### 2. A higher untrusted number does not authorize repair

PATH-057-C gives A trusted epoch 2 while B contains epoch 3 without trusted A-issued provenance.

Result: `Unresolved`.

PATH-057-H strengthens the witness by placing epoch 999999 at B without trusted A-issued provenance.

Result: A remains at epoch 2 and reconciliation stays `Unresolved`.

Therefore a larger numeric value does not acquire authority merely by being larger.

### 3. Coarse baseline: surviving modeled A-issued evidence can authorize repair

PATH-057-D gives A trusted epoch 2 while B remains at trusted epoch 1 in the same modeled domain.

Result: `AdvanceB 1 -> 2`.

PATH-057-E models A regressing to epoch 1 while B retains trusted evidence that epoch 2 was previously issued by A.

Result: `AdvanceA 1 -> 2`.

The repair is authorized by the surviving modeled A-issued evidence, not by numeric comparison alone.

This is the coarse PATH-057A baseline candidate. At this stage, `lineage` is only a shared modeled domain label; PATH-057B through PATH-057E later show that the label alone does not establish ancestry and therefore is not the final repair rule.

### 4. Absence of trusted provenance leaves disagreement unresolved

PATH-057-F gives B a higher epoch without trusted A-issued provenance.

Result: `Unresolved`.

PATH-057-G removes surviving epoch evidence from both systems.

Result: `Unresolved`.

The model does not reconstruct authority from payload values, effect counts, caller claims, or guessed successors.

### 5. Shared lineage label is not ancestry

PATH-057-I gives two trusted transitions the same lineage label but no parent relation between them.

Result: `Unresolved`.

A shared lineage identifier only denotes a modeled domain. It does not prove that one transition descends from another.

### 6. Explicit direct-parent evidence proves one hop in the tested candidate

PATH-057-J gives B transition 3 with a parent pointer to A transition 2.

Result: `AdvanceA 2 -> 3`.

PATH-057-K reverses the direction.

Result: `AdvanceB 2 -> 3`.

This establishes only the tested direct-parent case.

### 7. Forks and missing ancestry remain unresolved

PATH-057-L gives two transitions with the same parent but different transition identities.

Result: `Unresolved`.

PATH-057-M gives epochs 2 and 4, while epoch 4 points to missing transition 3.

Result: `Unresolved`.

A plausible numeric sequence does not substitute for missing ancestry evidence.

### 8. Repair application is monotonic

PATH-057-N applies a delayed repair for epoch 2 to state already at epoch 3.

Result: no downgrade.

A later repair to epoch 4 advances the state.

The tested repair helper therefore preserves monotonic state under delayed older repair input.

### 9. Complete trusted transition evidence can prove multi-hop ancestry in the tested candidate

PATH-057-O supplies the complete modeled chain:

`2 -> 3 -> 4`

All transitions are marked as trusted A issuances in the same lineage and the parent identities connect.

Result: `AdvanceA 2 -> 4`.

PATH-057-P removes the intermediate transition.

Result: `Unresolved`.

PATH-057-Q provides a numerically plausible sibling with the wrong transition identity instead of the required intermediate transition.

Result: `Unresolved`.

Thus the tested multi-hop candidate depends on the explicit transition chain, not only on epoch numbers.

### 10. The transition graph itself must be internally consistent

PATH-057-R constructs a cycle in the modeled transition graph.

Result: ancestry proof fails.

PATH-057-S provides two conflicting records with the same transition ID.

Result: ancestry proof fails.

PATH-057-T inserts a trusted-looking transition from a foreign lineage into an otherwise plausible chain.

Result: ancestry proof fails.

The strict candidate therefore refuses ancestry when the modeled transition graph is cyclic, ambiguous by transition identity, or crosses lineage domains.

## DERIVED

H28 is supported for the tested candidates:

Persistent A/B epoch disagreement cannot be reconciled safely from numeric epoch ordering alone.

The tested repair candidates require surviving evidence about modeled authority transitions.

The evidence becomes progressively stronger across PATH-057:

1. epoch number alone is insufficient
2. shared lineage label alone is insufficient
3. direct-parent evidence establishes only one-hop ancestry
4. complete trusted transition evidence can establish tested multi-hop ancestry
5. cyclic, ambiguous, incomplete, forked, or foreign-lineage evidence does not establish ancestry

The candidate repair rule is therefore evidence-directed rather than max-directed.

### Tested authorization rule

`AdvanceB` is permitted when the tested evidence establishes that B is behind a surviving modeled A-issued authority state.

`AdvanceA` is permitted when B retains surviving modeled evidence that A previously issued the higher state and the tested ancestry relation establishes that the evidence belongs to the same authority history.

Otherwise the result is `Unresolved`.

### Safety properties observed

Across the tested candidates:

- repair application does not downgrade trusted state
- bare numeric maximum is not promoted into authority
- the strict ancestry candidates PATH-057B through PATH-057E do not merge sibling or incompatible transition histories
- the strict ancestry candidates PATH-057B through PATH-057E do not fill missing ancestry from numeric plausibility
- reconciliation does not trigger an economic effect merely because authority state is repaired

## KNOWN FROM LITERATURE

Existing PKV research material already treats monotonically increasing epochs, fencing, stale state, authoritative state, and durable recovery history as established distributed-systems concepts.

PATH-057 uses that terminology as context only.

No literature source is claimed to prove this experiment, and no novelty claim is made.

## NOT ESTABLISHED

PATH-057 does not establish:

- behavior of real payment systems, APIs, databases, or providers
- cryptographic authenticity of transition evidence
- physical durability of the modeled evidence
- Byzantine fault tolerance
- consensus necessity
- quorum necessity
- two-phase-commit necessity
- a universally correct reconciliation architecture
- exactly-once semantics
- same-token idempotency
- resolution of FIND-029 historical-effect ambiguity
- that every real authority history must be represented as this parent-linked graph
- that this in-memory transition model is an appropriate production log
- novelty

The `trusted_origin` field is a test assumption, not proof of real authenticity.

## Candidate central result

In the tested reconciliation model, epoch magnitude alone cannot establish which persistent A/B state is authoritative.

Repair becomes justified only when surviving modeled evidence establishes a compatible authority-transition history for the state being installed.

A shared lineage label does not prove ancestry. Direct-parent evidence proves only the tested one-hop case. Multi-hop reconciliation requires the tested chain evidence to connect the transitions, while missing intermediates, sibling forks, cycles, conflicting transition identities, and foreign-lineage hops remain unresolved.

Therefore:

> reconciliation requires authority-transition evidence, not merely monotonic counters.

This statement is scoped to the tested PATH-057 candidates.

## Stop condition

PATH-057 reaches a new trust boundary:

The model assumes that authority-transition evidence can itself be trusted.

The experiment does not establish how such evidence becomes authentic, durable, uniquely identified, or recoverable after loss.

Building those properties would move the investigation from reconciliation policy into the authority-log / evidence-authenticity layer.

That is a separate PATH.

## Validation state before final sealing

At the end of PATH-057A through PATH-057E:

- targeted PATH-057 tests: 20 passed
- targeted failures: 0
- `git diff --check`: clean
- production `src/`: untouched
- no commit
- no push

Full-suite validation: 469 passed / 6 known unrelated failures (PATH-015/016/017/019/023/026).
