# FIND-003 — Manual Scores Are Not Economic Evidence

Status: DESIGN CORRECTION
Protocol: PP-0.1-draft

## Observation

The current experimental claim models accept normalized parameters such as:

Criticality = 0.90

Dependency Reach = 0.80

Continuity Responsibility = 0.85

These values are useful for synthetic testing, but the number itself does not explain how it was obtained.

If such values were used directly for real economic allocation, discretionary judgment could be hidden behind numerical precision.

## Principle

Every economically relevant parameter requires:

1. a definition;
2. observable inputs;
3. a deterministic measurement function;
4. inspectable evidence for those inputs;
5. a versioned normalization rule.

## Invariants

NO SCORED PARAMETER WITHOUT A MEASUREMENT FUNCTION.

NO MEASUREMENT WITHOUT OBSERVABLE EVIDENCE.

A normalized score is an output of measurement.

It is not economic evidence by itself.

## Required chain

Evidence
  ->
Observables
  ->
Measurement Function
  ->
Normalized Parameter
  ->
Claim
  ->
Allocation

Every transformation must be inspectable.

## First experiment

Criticality will be the first parameter moved from manual scoring into the Measurement Layer.

Candidate observables:

- downstream dependency count;
- blocking path count;
- failure blast radius;
- substitutability.

The initial normalization functions and weights are experimental hypotheses.

They are subject to pathological testing and protocol revision.

## UI consequence

Future controls must distinguish between:

RAW OBSERVABLES

MEASUREMENT RULES

PROTOCOL WEIGHTS

CALCULATED SCORES

A slider must never make a manual opinion look like observed evidence.

## Documentation impact

Any study language implying that normalized parameter values are directly observed facts must be corrected.

The study must distinguish raw evidence, observables, normalization and calculated scores.
