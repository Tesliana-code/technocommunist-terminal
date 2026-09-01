# FIND-005 — Weak Signals Must Not Accumulate Into False Systemic Importance

Status: OBSERVED
Protocol: PP-0.1-draft
Evidence: PATH-015

## Observation

The dependency qualification layer successfully prevented irrelevant dependencies from creating economic weight.

PATH-013 demonstrated:

10,000 dependencies with zero qualified relevance
produce aggregate dependency weight:

0.000000

PATH-014 demonstrated:

a small number of strongly evidenced dependencies
can produce high aggregate dependency weight.

However, PATH-015 exposed a second-order failure in the aggregation function.

The experimental aggregation rule was:

Aggregate = 1 - product(1 - dependency_score)

For 1,000 individually weak dependencies, each scoring:

0.010000

the aggregate dependency weight became:

0.999957

## Failure

The aggregation function interprets repeated weak signals as overwhelming systemic evidence.

This permits a weak-edge swarm to approximate maximum dependency importance even when no individual dependency demonstrates meaningful criticality.

The function is mathematically bounded.

It is not economically safe.

## Finding

WEAK SIGNAL VOLUME MUST NOT CONVERGE TO SYSTEMIC IMPORTANCE BY COUNT ALONE.

Qualification of individual dependencies is necessary but insufficient.

Aggregation itself is part of the economic measurement model and must survive adversarial testing.

## Why this matters

A system may naturally contain hundreds or thousands of low-strength relationships.

Examples include:

- optional consumers;
- telemetry paths;
- convenience integrations;
- duplicated graph edges;
- indirect references;
- low-impact automation;
- generated artifacts;
- structurally correlated dependencies.

Their existence does not imply that removing the measured work would cause systemic degradation.

A protocol that allows weak-edge accumulation to approach 1.0 creates an economic incentive for complexity.

That incentive is unacceptable.

## Counterfactual interpretation

Criticality should approximate:

system degradation if the work or capability disappears

rather than:

number of relationships that can be associated with the work.

This suggests that dependency aggregation may eventually require structural or counterfactual information.

## Candidate aggregation hypotheses

No candidate is accepted yet.

### H-A — Qualification threshold

Dependencies below a minimum evidence strength do not participate in aggregation.

Risk:

A hard threshold may discard meaningful combinations of weak signals.

### H-B — Ranked diminishing contribution

Sort dependencies by qualified strength.

Each additional dependency contributes less than the previous one.

Risk:

The decay function itself becomes a protocol parameter.

### H-C — Dependency clustering

Correlated dependencies are grouped before aggregation.

Many edges representing the same underlying dependency structure contribute approximately once.

Risk:

Cluster identity must itself be evidenced and deterministic.

### H-D — Independence-aware aggregation

Accumulation is permitted only when evidence supports that dependencies represent distinct failure or value paths.

Risk:

Independence may be difficult to establish from available evidence.

### H-E — Counterfactual graph measurement

Estimate criticality from system degradation after removing the artifact or capability from the dependency graph.

Risk:

Requires a sufficiently expressive and trustworthy graph model.

## Experimental requirement

The next implementation must compare multiple aggregation hypotheses against the same pathological suite.

At minimum, a candidate must preserve:

PATH-013:
irrelevant dependency swarm -> approximately zero

PATH-014:
few strong dependencies > many irrelevant dependencies

PATH-015:
weak dependency swarm must not approach systemic criticality

No aggregation rule becomes protocol policy merely because it passes one test.

## New measurement principle

THE AGGREGATOR IS PART OF THE CLAIM.

Therefore:

aggregation rules must be versioned;

aggregation assumptions must be inspectable;

aggregation parameters must be explicit;

aggregation behavior must be adversarially tested.

## Economic consequence

Complexity must not manufacture entitlement.

Creating more edges, events, artifacts, references, commits, tickets or dependency relationships must not increase an economic claim unless those additions represent evidenced economic structure.

## UI consequence

The future Distribution Machine instrument should expose aggregation reasoning.

Example:

DEPENDENCY MEASUREMENT

Raw edges                  1,247
Qualified edges               38
Independent paths              4
Strong dependencies            2
Weak dependencies             32
Rejected noise             1,209

Aggregation model:
EXPERIMENTAL / H-B

Aggregate dependency weight:
0.61

WHY?

[inspect qualification]
[inspect clusters]
[inspect aggregation]
[run counterfactual]

The displayed score must never conceal the structure that produced it.

## Protocol consequence

PP-0.1 must not adopt the current complementary accumulation function as an economic rule.

Its failure under PATH-015 is now part of the executable evidence record.

## Next experiment

Implement competing aggregation strategies behind explicit experimental identities.

Run the same dependency fixtures through each candidate.

Compare:

- resistance to weak-edge farming;
- preservation of strong dependencies;
- monotonicity;
- boundedness;
- sensitivity to duplicate edges;
- sensitivity to correlated edges;
- interpretability.

Selection must follow evidence from the experiment.

It must not precede it.
