# FIND-002 — Stewardship Must Be a Separate Economic Claim

Status: OBSERVED
Protocol: PP-0.1-draft
Evidence: PATH-004, PATH-008, PATH-009

## Observation

The original Contribution Value formula included Stewardship as one of its weighted factors.

At the same time, PP-0.1 defined a separate Stewardship Pool.

This allowed the same stewardship property to create economic value twice:

1. once by increasing the contributor's share of the Contribution Pool;
2. again through the separate Stewardship Pool.

## Experimental result

PATH-008 isolated the stewardship variable.

Two otherwise identical Contribution claims were compared:

Stewardship = 0.0

and

Stewardship = 1.0

Under the original Contribution formula, changing only Stewardship increased Contribution Value by approximately 13.89%.

This demonstrated a separate-claim violation.

## Original model

Contribution Value included:

Responsibility

Criticality

Deal Impact

Stewardship

Dependency Reach

Evidence Confidence

Because Stewardship already had its own economic claim class, including it here created double-claim leakage.

## Corrected Contribution model

Contribution Value now uses:

Responsibility

Criticality

Deal Impact

Dependency Reach

Evidence Confidence

Stewardship remains observable data but does not alter Contribution Value.

Current experimental weights:

Responsibility = 0.20

Criticality = 0.30

Deal Impact = 0.25

Dependency Reach = 0.25

These weights are experimental parameters, not settled policy.

## Separate Stewardship model

Stewardship is now evaluated through its own claim path using:

Continuity Responsibility

Maintenance Burden

Failure Exposure

Dependency Reach

Evidence Confidence

Its allocation comes exclusively from the Stewardship Pool.

## Verification

PATH-004 verifies that changing Stewardship alone does not change Contribution Pool allocation.

PATH-008 verifies that Stewardship no longer changes Contribution Value.

PATH-009 verifies that two people may have equal Contribution claims while having materially different Stewardship claims and allocations.

## Invariant

CONTRIBUTION CLAIM != STEWARDSHIP CLAIM

and

STEWARDSHIP FACTORS MUST NOT INCREASE CONTRIBUTION ALLOCATION

A person may legitimately receive allocations from both pools only when each allocation is supported by its own claim path and evidence.

## Protocol consequence

Economic dimensions with separate claim classes must have separate allocation paths.

A property cannot silently increase one claim while also generating entitlement through another pool.

## Documentation impact

Any PP-0.1 formula that includes Stewardship inside Contribution Value must be corrected.

The study must present Contribution and Stewardship as independent economic claims with independent formulas, evidence and allocations.

## NCR-SIM-001 evidence

The NCR simulation demonstrates the separation clearly.

Operations receives a relatively smaller Contribution allocation because its evidenced contribution claim is smaller relative to the other contribution claims.

At the same time, Operations receives the largest Stewardship allocation because its synthetic production-continuity claim carries the strongest ongoing stewardship factors.

The model can therefore express:

historical/product contribution

and

ongoing responsibility for preserving value

without forcing them into one score.

## Visualization candidate

                         ECONOMIC VALUE
                              |
             +----------------+----------------+
             |                                 |
             v                                 v
      CONTRIBUTION CLAIM                STEWARDSHIP CLAIM
             |                                 |
   RR + CC + DI + DR + E             CR + MB + FE + DR + E
             |                                 |
             v                                 v
    CONTRIBUTION POOL                 STEWARDSHIP POOL
          €10,500                           €6,000
             |                                 |
             +---------------+-----------------+
                             |
                             v
                    PERSON / STATEMENT

The two paths may terminate at the same person.

They must never become the same claim.
