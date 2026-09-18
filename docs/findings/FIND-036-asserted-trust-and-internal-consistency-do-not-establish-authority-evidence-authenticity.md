# FIND-036 — Asserted Trust and Internal Consistency Do Not Establish Authority-Evidence Authenticity

Status: OBSERVED / DERIVED / NOT ESTABLISHED
Protocol: PP-0.1-draft
Evidence: PATH-058
Canonical prior commit: PATH-057 / FIND-035 at `3e33d35`

## Hypothesis under test

FIND-035 established a new trust boundary.

Persistent epoch disagreement could not be reconciled from numeric order alone. The tested candidate instead depended on surviving authority-transition evidence.

PATH-058 attacks the next assumption:

Can authority-transition evidence establish its own authenticity from its visible fields and internal structure, or does authentication require additional evidence outside the object being authenticated?

The tested models are test-local only.

## Test-local ground truth

PATH-058 introduces `Path058EvidenceReality` with two test-oracle states:

- `Genuine`
- `Forged`

This ground-truth distinction is deliberately unavailable to reconciliation code.

It exists only so the experiment can compare:

1. what the reconciliation candidate can observe
2. what the modeled world says is actually correct

No production authenticity mechanism is modeled by this enum.

## OBSERVED

### 1. An explicit untrusted marker is rejected

PATH-058-A provides a structurally plausible transition whose visible `trusted_origin` field is false.

Result:

`Unresolved`

This is the control case.

The PATH-057 candidate refuses evidence that openly identifies itself as outside the modeled trusted issuance boundary.

### 2. A forged trusted marker is observationally accepted like genuine evidence

PATH-058-B constructs a transition whose visible fields claim:

- modeled canonical issuer A
- expected lineage
- expected parent transition
- `trusted_origin: true`

The test oracle separately marks the record as forged.

The PATH-057 direct-ancestry candidate sees only the visible fields and returns:

`AdvanceA 2 -> 3`

The ground-truth oracle requires:

`Unresolved`

Therefore the visible trust assertion is not itself authentication.

### 3. Genuine and forged worlds can be observationally identical

PATH-058-C gives reconciliation code the same visible transition in two modeled worlds.

World G:

- visible evidence is genuine
- correct verdict is `AdvanceA 2 -> 3`

World F:

- visible evidence is forged
- correct verdict is `Unresolved`

The observable evidence supplied to reconciliation is identical.

The reconciliation candidate therefore returns the same result in both worlds.

This establishes the tested identifiability boundary:

> If authenticity is not represented in the observable input, the existing observable fields cannot distinguish genuine evidence from a forged copy carrying the same claims.

The ground-truth distinction is external to the reconciliation input.

### 4. Internal chain consistency does not authenticate chain origin

PATH-058-D inserts a forged intermediate transition into a parent-linked chain whose visible fields are otherwise internally consistent.

The strict PATH-057 ancestry validator accepts the chain.

Its checks establish properties such as:

- trusted-looking modeled issuer field
- matching lineage
- matching transition identities
- parent linkage
- monotonic epoch relation
- structural consistency

Those properties do not expose the test-oracle fact that the intermediate record is forged.

Therefore:

> structural consistency is not authenticity evidence by itself.

### 5. A separate modeled authority record can verify a matching transition under an assumed trust root

PATH-058-E adds a separate `Path058AuthorityRecord`.

A visible transition whose identifying contents exactly match the modeled authority record is accepted by the verification candidate.

This adds information that was absent from PATH-058A.

The authority record is assumed trustworthy in this candidate.

That assumption is exogenous.

### 6. A mismatching visible transition is rejected against the surviving authority record

PATH-058-F keeps a canonical authority record for epoch 3 while presenting a visible transition claiming epoch 4 under the same modeled transition identity.

Result:

verification fails.

The separate authority record can therefore distinguish this tested mismatch.

This does not establish that every possible forgery is detectable.

It establishes only the tested comparison rule.

### 7. Loss of the authority record removes the tested authenticity witness

PATH-058-G presents a trusted-looking visible transition but removes the separate authority record.

Result:

verification fails.

This result means authentication is unavailable in the tested candidate.

It does not prove that the visible record is forged.

Absence of the witness is not evidence of falsity.

### 8. A poisoned authority record can authenticate the wrong history

PATH-058-H constructs a forged visible transition and a modeled authority record containing the same forged contents.

The verification candidate accepts the match.

The test oracle still marks the history as forged.

Therefore the separate authority registry does not eliminate the trust boundary.

It relocates it.

The verification result is only as sound as the authority record against which evidence is compared.

## DERIVED

H29 is supported for the tested candidates:

An asserted trust marker and internally consistent authority-transition structure do not establish authenticity when forged and genuine worlds can expose identical observable evidence.

Adding a separate modeled authority record can resolve tested mismatches only by introducing a new exogenous trust assumption.

The authenticity problem is therefore not removed.

It is relocated to the source, integrity, availability, and recovery semantics of the authority record.

### Tested distinction

PATH-057 establishes structural questions such as:

- does this transition descend from that transition?
- is the modeled lineage compatible?
- is the parent chain complete?
- is the graph internally consistent?

PATH-058 establishes a different question:

- did the claimed authority actually issue this evidence?

The first class can be checked from structure in the tested model.

The second cannot be derived from the same structure when a forged object can reproduce those visible fields.

### Epistemic boundary

The critical PATH-058-C pair has:

same observable evidence

but:

different correct reconciliation verdicts under the modeled authenticity ground truth

Therefore no decision function restricted to those same observable fields can distinguish the two modeled worlds.

New information is required, or the result must remain unresolved.

### Trust-root relocation

The PATH-058B candidate supplies that new information through a separate modeled authority record.

Its behavior is:

- matching authority record -> tested authentication succeeds
- mismatching authority record -> tested authentication fails
- missing authority record -> authentication unavailable
- poisoned authority record -> wrong history can be accepted

Thus:

> comparison with a trust root does not authenticate the trust root itself.

## Safety interpretation

The tested candidate supports conservative behavior when the authority witness is absent or mismatched.

It does not justify interpreting missing authority evidence as proof of forgery.

It also does not justify accepting a matching authority record unless the model separately grants that record authority.

PATH-058 therefore separates:

- structural validity
- authenticity
- authority-record availability
- authority-record integrity

These are distinct properties.

## KNOWN FROM LITERATURE

Existing distributed-systems and security literature contains established concepts involving trusted roots, authenticated state, tamper resistance, durable logs, signatures, and recovery provenance.

PATH-058 uses generic trust-boundary terminology only.

No external source is claimed to prove this experiment.

No novelty claim is made.

## NOT ESTABLISHED

PATH-058 does not establish:

- cryptographic authenticity
- digital signatures
- PKI requirements
- key-management requirements
- hardware roots of trust
- append-only log requirements
- consensus requirements
- quorum requirements
- Byzantine fault tolerance
- physical durability
- tamper resistance
- collision resistance
- real-world identity verification
- secure recovery of lost authority records
- correctness of any production provider
- a universally correct authenticity architecture
- exactly-once execution
- resolution of FIND-029 historical-effect ambiguity
- that one centralized authority registry is required
- novelty

`Path058AuthorityRecord` is test-local modeled state.

Its authority is assumed, not derived.

## Candidate central result

In the tested authority-evidence model, visible claims of trusted origin and internally consistent ancestry do not establish authenticity.

A forged and a genuine transition can expose identical observable evidence while requiring different correct reconciliation verdicts under the modeled authenticity ground truth.

A separate modeled authority record can distinguish tested mismatches, but only by introducing a new trust root.

If that record is absent, authentication becomes unavailable.

If that record is corrupted, the tested verifier can accept the wrong history as authenticated within the model.

Therefore:

> authority evidence cannot authenticate itself merely by asserting trust or forming a consistent chain.

And:

> introducing an authenticity witness relocates the trust boundary to that witness.

These statements are scoped to the tested PATH-058 candidates.

## Stop condition

PATH-058 reaches the next trust boundary:

The candidate assumes the separate authority record is itself sound and recoverable.

The experiment does not establish:

- how that record becomes durable
- how corruption is detected
- how uniqueness is preserved
- how authority history is recovered after loss
- which surviving source is authoritative during recovery

Those are authority-record durability and recovery-provenance questions.

They belong to a separate PATH.

## Validation state before final sealing

At the end of PATH-058A through PATH-058B:

- targeted PATH-058 tests: 8 passed
- targeted failures: 0
- `git diff --check`: clean
- production `src/`: untouched
- no commit
- no push

Full-suite validation: library unit tests 47/47 passed; NCR simulation 2/2 passed; pathological_cases 477 passed / 6 known unrelated failures (PATH-015/016/017/019/023/026). No new PATH-058 failure.
