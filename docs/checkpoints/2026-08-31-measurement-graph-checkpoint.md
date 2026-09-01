# Distribution Machine — Measurement / Graph Checkpoint

Date: 2026-08-31
Protocol status: PP-0.1 experimental
State: laboratory checkpoint

## Current economic model

Distribution Machine separates:

- Labor
- Capital
- Contribution
- Stewardship

Core invariant:

Capital != Labor != Contribution != Stewardship

Labor remains a contractual obligation before distributable surplus.

## Measurement principle

No economically relevant score should exist without:

1. a definition
2. observable inputs
3. a deterministic measurement function
4. inspectable evidence
5. a versioned normalization rule

Pipeline:

Evidence
-> Observables
-> Measurement Function
-> Normalized Parameter
-> Claim
-> Allocation

## Dependency / graph discoveries

The dependency measurement experiments established that raw counts,
anonymous scores, labels, and IDs are insufficient evidence of
economic importance.

Observed progression:

DependencyId
-> structural relation
-> dependency path
-> failure domain
-> graph-derived independence

### Confirmed adversarial results

PATH-015
Many weak dependencies can accumulate into false systemic importance.
Complementary aggregation failed.

PATH-016
Hard thresholds create disproportionate economic cliffs.
Thresholded aggregation failed.

PATH-017
Duplicating anonymous dependency records creates value.
Anonymous RankedDecay failed.

PATH-018
Exact dependency-ID deduplication prevents exact duplicate inflation.
PASS.

PATH-019
Renaming the same structural dependency defeats ID-only deduplication.
FAIL.

PATH-020
Structural deduplication resists dependency-ID laundering.
PASS.

PATH-021
Multiple relationship descriptions of the same underlying path must not
create independent economic paths.
PASS.

Observed:
one path = 0.600000
Runtime + DataFlow descriptions = 0.600000

PATH-022
Two genuinely distinct declared dependency paths remain distinct.
PASS.

Observed:
one path = 0.600000
two paths = 0.900000

PATH-023
Distinct path IDs sharing the same downstream bottleneck are incorrectly
treated as independent.
EXPECTED ADVERSARIAL FAIL.

Observed:
one path = 0.600000
two declared paths through same bottleneck = 0.900000

PATH-024
Explicit shared failure-domain grouping collapses correlated paths.
PASS.

Observed:
one path = 0.600000
two paths in same failure domain = 0.600000

PATH-025
Distinct failure domains preserve independent economic signal.
PASS.

Observed:
one domain = 0.600000
two domains = 0.900000

PATH-026
Failure-domain IDs can themselves be laundered.

EXPECTED ADVERSARIAL FAIL.

Observed:
real bottleneck = 0.600000
same bottleneck under 100 failure-domain IDs = 1.000000

## Current conclusion

Failure independence cannot be established by identifiers alone.

Candidate invariant:

INDEPENDENCE IS A PROPERTY OF GRAPH STRUCTURE, NOT OF LABELS.

FailureDomainId may remain useful metadata or provenance, but must not be
authoritative evidence that two paths are economically independent.

## Next technical experiment

Build a minimal explicit dependency graph.

First graph-derived adversarial case:

    A
   / \
  B   C
   \ /
    D

The measurement engine must derive the shared structural bottleneck
without relying on manually declared FailureDomainId values.

Then compare against a genuinely independent topology:

A -> B -> D
A -> C -> E

Future work should test:

- shared-node bottlenecks
- shared-edge bottlenecks
- node-disjoint paths
- edge-disjoint paths
- correlated failure domains
- cycles
- graph isomorphism
- path-label laundering
- graph-derived counterfactual removal

## Documentation / product direction

After repository publication:

- protocol specification
- measurement specification
- findings register
- invariants
- economic-flow diagrams
- value-flow diagrams
- employee-facing contribution guide
- contribution forecast / simulation model
- evidence and provenance guide
- dispute / correction procedure
- protocol versioning rules
- Ratatui economic debugger specification

Employee-facing goal:

A worker should be able to understand how durable, high-quality,
evidence-backed work may affect future distributable claims without
requiring opaque managerial scoring.

The system should reward durable contribution rather than raw activity volume.

## Ratatui direction

Ratatui remains the intended terminal interface.

The UI should eventually expose:

- Pools
- Claims
- Measurements
- Simulation
- evidence drill-down
- graph inspection
- sensitivity analysis
- pathological test status
- allocation explanation
- protocol sandbox diffs

Production protocol state must remain immutable from experimental UI controls.

## Immediate next step

Do not repair PATH-026 by adding another authoritative identifier.

Proceed with graph-derived structural independence.
