# FIND-004 — Raw Counts Are Gameable Observables

Status: OBSERVED
Protocol: PP-0.1-draft
Evidence: PATH-012

## Observation

The first Criticality measurement experiment used raw downstream dependency count as an observable.

The normalization rule saturated dependency count at 10.

This appeared bounded and safe because the normalized value could not exceed 1.0.

PATH-012 demonstrated that the observable itself was still gameable.

A synthetic artifact with:

10,000 downstream dependencies

0 blocking paths

0 failure blast radius

easy substitutability

still received a Criticality score of:

0.3000

solely because the dependency-count dimension carried 30% of the Criticality measurement weight.

## Failure mode

A raw count can be increased without increasing economic importance.

This creates an incentive to manufacture:

- trivial dependencies;
- artificial coupling;
- unnecessary integration points;
- fragmented architecture;
- dependency noise.

A bounded normalization function limits numerical explosion.

It does not make the underlying observable meaningful.

## Correction

Criticality no longer consumes raw downstream dependency count.

The model now consumes:

Qualified Dependency Weight

The value must be between 0.0 and 1.0.

Trivial or irrelevant dependencies contribute zero qualified dependency weight.

## Important limitation

Qualified Dependency Weight is currently still an experimental normalized input.

The model has moved one level closer to meaningful measurement, but the value itself must eventually be derived from inspectable dependency evidence.

Future work must define how individual dependencies become qualified.

Candidate evidence dimensions include:

- blocking relevance;
- failure propagation;
- operational reliance;
- substitutability;
- business-path relevance;
- dependency strength.

## Invariant

COUNT != IMPACT

and

RAW ACTIVITY VOLUME MUST NOT CREATE ECONOMIC VALUE BY ITSELF.

## Measurement consequence

A useful observable must resist trivial inflation.

Metrics must represent economically meaningful structure rather than activity volume.

## Protocol consequence

Normalization is not sufficient protection against metric gaming.

The semantic quality of the observable must be tested independently from the mathematical bounds of the normalization function.

## Documentation impact

Any PP-0.1 language treating raw dependency count as a direct measure of Criticality must be corrected.

The study must distinguish:

raw graph edges

from

qualified economically relevant dependencies.

## UI consequence

The future terminal instrument should never display:

Dependencies: 47 -> Criticality 0.83

without showing why those dependencies qualified.

A better inspection path is:

47 raw dependencies
        |
        v
12 qualified dependencies
        |
        v
qualified dependency evidence
        |
        v
measurement function
        |
        v
Criticality contribution

## Visualization candidate

RAW DEPENDENCIES
      10,000
         |
         v
   qualification
         |
    +----+----+
    |         |
trivial   economically
noise      relevant
    |         |
    v         v
   0       weighted
 impact     evidence
              |
              v
        CRITICALITY INPUT

The graph may contain many edges.

Only evidenced relevant edges may affect the economic claim.
