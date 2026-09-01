# FIND-007 — Duplicate Edges Must Not Create Economic Importance

Status: OBSERVED
Protocol: PP-0.1-draft
Evidence: PATH-017

## Observation

The RankedDecay aggregation candidate was resistant to large swarms of very weak dependencies.

However, PATH-017 duplicated the same dependency 100 times.

One dependency scored:

0.600000

The duplicated set aggregated to:

1.000000

## Finding

An aggregator operating only on anonymous numeric dependency scores cannot distinguish:

100 distinct economically meaningful dependencies

from

the same dependency represented 100 times.

Therefore duplicate representation can manufacture systemic importance.

## Principle

REPRESENTATION MULTIPLICITY MUST NOT CREATE ECONOMIC VALUE.

One economic relationship may appear in multiple technical records.

Its claim weight must depend on the relationship itself, not on how many times it is represented.

## Architectural consequence

Dependency aggregation requires identity and structure.

At minimum, future dependency records may require:

- dependency identity;
- source capability;
- target capability;
- relationship type;
- evidence provenance;
- structural path or cluster identity.

Aggregation over Vec<f64> is insufficient for economic use.

## Measurement consequence

Before aggregation, dependencies must support deterministic deduplication or structural equivalence analysis.

## Counterfactual interpretation

The relevant question is:

How much distinct system reliance disappears if this capability disappears?

Repeated descriptions of the same reliance do not increase the answer.

## Protocol consequence

RankedDecay is not accepted as a PP-0.1 aggregation rule in its current anonymous-score form.

Its decay behavior may remain experimentally useful after structural deduplication is introduced.

## UI consequence

Future inspection should expose both technical records and economically distinct relationships.

Example:

Raw dependency records:        147
Unique technical edges:         39
Distinct economic paths:         6
Duplicate representations:     108

Aggregation must operate on the economically meaningful structure.

## Next architectural experiment

Introduce explicit dependency identity and graph structure.

Then test:

- exact duplicate removal;
- same source/target under different evidence records;
- correlated paths;
- independent paths;
- cycles;
- shared downstream bottlenecks.

Only after structural identity exists should aggregation candidates return to the tournament.
