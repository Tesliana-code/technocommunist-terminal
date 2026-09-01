# FIND-006 — Threshold Cliffs Amplify Small Evidence Changes

Status: OBSERVED
Protocol: PP-0.1-draft
Evidence: PATH-016

## Observation

The Thresholded aggregation candidate rejected dependencies below a fixed qualification threshold of 0.20.

PATH-016 compared two nearly identical dependencies:

0.199
0.201

The evidence delta was:

0.002

The aggregation output changed from:

0.000

to:

0.201

The output delta was:

0.201

## Finding

A hard qualification threshold can transform a very small evidence change into a large economic change.

This introduces discontinuity into the claim function.

## Risk

If economic allocation depends on the result, tiny measurement corrections near the threshold may produce disproportionate changes in money.

This creates:

- boundary gaming;
- dispute pressure around cutoff values;
- instability under small evidence corrections;
- incentives to optimize for threshold crossing.

## Principle

SMALL EVIDENCE CHANGES SHOULD NOT CAUSE DISPROPORTIONATE ECONOMIC JUMPS WITHOUT AN EXPLICIT STRUCTURAL REASON.

## Protocol consequence

A fixed hard threshold is not accepted as the default dependency aggregation mechanism for PP-0.1.

Thresholds may still exist as evidence-quality gates, but their economic effect must be explicitly justified and tested.

## Measurement consequence

Continuity and sensitivity must be properties tested alongside boundedness and anti-gaming behavior.

## UI consequence

Future simulation tooling must display sensitivity around protocol boundaries.

A useful diagnostic should answer:

WHAT HAPPENS IF THIS EVIDENCE CHANGES BY +/- 0.01?

Economic discontinuities must be visible before protocol adoption.
