# FIND-001 — Capital Pool Remainder Must Remain Explicit

Status: OBSERVED
Protocol: PP-0.1-draft
Evidence: PATH-010, PATH-011

## Observation

A Capital Pool may be larger than the total verified outstanding capital claims.

Synthetic case:

Capital Pool = €7,500

Verified capped capital claim = €1,000

Capital Allocation = €1,000

Capital Remainder = €6,500

## Formula

Capital Allocation =
min(Capital Pool, Total Verified Outstanding Capital Claims)

Capital Remainder =
Capital Pool - Capital Allocation

For the synthetic case:

Capital Allocation =
min(€7,500, €1,000)
= €1,000

Capital Remainder =
€7,500 - €1,000
= €6,500

## What the experiment showed

The original allocator treated the Capital Pool as money that had to be fully distributed.

PATH-010 demonstrated that this allowed a claimant with a verified capped claim of €1,000 to receive €7,500.

That violated the economic meaning of the return cap.

The allocator was corrected so that capital payout cannot exceed the total verified capped claims.

PATH-011 then demonstrated that the remaining €6,500 stays outside claimant allocations.

## Protocol consequence

A pool percentage defines the maximum amount made available to a claim class.

It does not create additional entitlement when valid claims are smaller than the pool.

Unused Capital Pool value must therefore remain explicitly accounted for.

It must not automatically:

- increase a capital claimant's return;
- become founder compensation;
- enter the Contribution Pool;
- enter the Stewardship Pool;
- disappear from reconciliation.

Its eventual disposition requires an explicit protocol rule.

## Invariant

CAPITAL PAYOUT <= VERIFIED CAPPED CAPITAL CLAIM

and

CAPITAL POOL =
CAPITAL ALLOCATION + CAPITAL REMAINDER

## Governance consequence

Unclaimed economic value requires explicit disposition.

Silence is not a distribution rule.

## Documentation impact

Any PP-0.1 text stating or implying that the entire Capital Pool is always distributed must be corrected.

The protocol must distinguish:

Capital Pool

Capital Allocation

Capital Remainder

The destination of Capital Remainder remains an open governance decision.

## Visualization candidate

€7,500 CAPITAL POOL
        |
        v
Verified outstanding claims?
        |
        +---- €1,000 ----> CAPITAL ALLOCATION
        |
        +---- €6,500 ----> CAPITAL REMAINDER
                              |
                              v
                     EXPLICIT DISPOSITION
                     REQUIRED BY PROTOCOL
