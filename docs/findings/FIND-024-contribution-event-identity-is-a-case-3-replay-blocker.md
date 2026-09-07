# FIND-024 — `ContributionEvent` Has No Authoritative Identity: Replay Protection Is a CASE 3 Blocker

Status: OBSERVED / DERIVED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-046.5

## Mission type

Bounded production remediation (not a pure research experiment). This
mission was authorized to modify production code, narrowly, to fix two
concrete defects PATH-046/FIND-023 confirmed: non-finite numeric input,
and duplicate/replay contribution events.

## Part A — non-finite value defect: FIXED

### Root cause (confirmed)

`base_work_value < 0.0` (and the identical `value < 0.0 || value > 1.0`
pattern in `ContributionWeights::validate`/`ContributionFactors::validate`)
never catches `NaN`, because IEEE-754 comparisons against NaN are always
`false`. There was no upper bound or finiteness check anywhere in this
pipeline. A single non-finite `base_work_value`, factor, weight, or pool
value silently propagated through `evidence_adjusted_value` and
`total_contribution`, producing `Ok(...)` allocations with `NaN` shares
for every contributor (or `0.0`/`NaN` splits in the infinity case) —
confirmed directly against production by PATH-046-M before this fix.

### Patch (production)

Four call sites, all pre-existing `validate()`/`calculate()`/
`allocate_contribution_pool` functions — no new functions, no
restructuring:

- `ContributionWeights::validate` (`src/contribution.rs`): reject any
  non-finite weight before the existing range check —
  `Err("contribution weights must be finite")`.
- `ContributionFactors::validate` (`src/contribution.rs`): reject any
  non-finite factor (including `evidence_confidence`) before the
  existing range check — `Err("contribution factors must be finite")`.
- `ContributionEvent::calculate` (`src/contribution.rs`): reject a
  non-finite `base_work_value` before the existing negativity check —
  `Err("base work value must be finite")`.
- `allocate_contribution_pool` (`src/distribution.rs`): reject a
  non-finite `contribution_pool` before the existing negativity check —
  `Err("contribution pool must be finite")` (needed because
  `allocated_amount = pool * normalized_share` could still be poisoned
  by a non-finite pool even with perfectly finite shares).

Since every previously-valid input was already finite (bounded to
`[0,1]` for weights/factors, or simply not NaN/Infinity for
`base_work_value`/pool in any existing test or realistic caller), this
patch is purely additive rejection — no previously-accepted input is now
rejected, and no allocation formula changed. Confirmed by re-running all
47 pre-existing unit tests unchanged (`cargo test --lib`) and by a
dedicated backward-compatibility test
(`path_046_5_backward_compatible_scale_invariance_ordering_and_conservation`).

### Behavior before / after

| Input | Before | After |
|---|---|---|
| `base_work_value = NaN` | `Ok(...)`, every share `NaN` | `Err("base work value must be finite")` |
| `base_work_value = +Infinity` | `Ok(...)`, that contributor's share `NaN`, others `0.0` | `Err("base work value must be finite")` |
| `base_work_value = -Infinity` | `Err("base work value cannot be negative")` (caught, coincidentally, by the pre-existing check) | `Err("base work value must be finite")` (caught earlier, same outcome class) |
| `contribution_pool = NaN`/`Infinity` | `Ok(...)` with poisoned `allocated_amount` | `Err("contribution pool must be finite")` |
| any valid finite input | unchanged | unchanged |

### Tests

`path_046_5_a1` through `a7` (`tests/pathological_cases.rs`), plus a
dedicated backward-compatibility test. `path_046_m` (the original
PATH-046 test that documented the pre-fix defect) was updated in place
to assert the new, correct rejection behavior, with a comment recording
its history — it is no longer accurate to describe production as
accepting NaN/Infinity, so leaving it asserting the old behavior would
have been a stale, false regression guard.

## Part B — replay/duplicate defect: BLOCKED (CASE 3), not fixed

### Identity inventory (required before any edit)

`ContributionEvent { contributor_id: String, contribution_id: String,
base_work_value: f64, factors: ContributionFactors }`
(`src/contribution.rs`).

1. **Fields**: as above — no other fields exist.
2. **Constructed**: only at call sites the caller controls (test modules
   in `distribution.rs`/`contribution.rs`, and this experiment's
   `tests/pathological_cases.rs` helpers). No production ingestion
   pipeline exists in this repository at all — `ContributionEvent`
   values are always directly constructed by whatever code calls
   `allocate_contribution_pool`.
3. **Can multiple legitimate events exist for the same contributor?**
   Yes — nothing prevents, and existing tests (`allocated_amounts_
   reconcile_to_entire_pool` and this mission's own
   `path_046_5_distinct_identical_valued_events_both_survive`) rely on,
   one contributor submitting more than one `ContributionEvent`.
4. **Is there already a unique event ID, global or source-scoped?**
   `contribution_id: String` is the only field that could plausibly
   serve this role. It is NOT validated for uniqueness anywhere, has NO
   doc comment establishing "same id ⇒ same economic event" semantics,
   and `allocate_contribution_pool` never inspects it for anything
   except copying it into the output — it is used purely as a
   human-readable label in every existing call site (`"capability-a"`,
   `"unverified-claim"`, `"future-capability"`, ...).
5. **Externally supplied, locally generated, or derived?** Externally
   supplied by the caller, with no format, generation, or uniqueness
   contract documented or enforced anywhere.
6. **Is uniqueness currently enforced anywhere?** No — confirmed by
   inspection of `allocate_contribution_pool`'s full body: it iterates,
   scores, sums, and normalizes; it never groups, compares, or filters
   by `contribution_id` (or any other field) at all.
7. **Aggregation before or inside allocation?** Inside —
   `allocate_contribution_pool` computes `total_contribution` as a
   direct sum over whatever `events` slice it is given; there is no
   separate aggregation stage upstream in production.
8. **Can replay occur at multiple layers?** Only one layer exists in
   this repository: the caller assembling the `events` slice passed
   directly into `allocate_contribution_pool`. There is no transport,
   queue, or ingestion layer in this codebase to reason about
   separately.
9. **Narrowest authoritative boundary for rejecting invalid numeric
   input**: `ContributionEvent::calculate` (per-event) and
   `allocate_contribution_pool` (for the pool value) — exactly where
   Part A's patch was applied.
10. **Narrowest authoritative boundary for replay protection, if one
    existed**: would also be `allocate_contribution_pool` (the one place
    all events for one allocation are seen together) — but see below,
    this boundary has nothing authoritative to key on.

### Classification: CASE 3

No field of `ContributionEvent` — alone or in any combination — carries
documented, enforced "same identity ⇒ same real-world economic event"
semantics. `contribution_id` *looks* like it could be an identity but is
exactly the same shape FIND-016/019 already characterized: a bare,
self-issued string with no issuer/authority binding. Using it (or
`contributor_id` + value, or full-payload equality) as a dedup key would
be inventing a behavioral convention with no evidence backing it —
explicitly forbidden by this mission ("Forbidden replay keys": exactly
this list). **No deduplication was implemented anywhere in production.**

### Why legitimate identical-valued events remain safe

Because nothing was implemented, no legitimate event can be dropped.
`path_046_5_distinct_identical_valued_events_both_survive` pins this as
an explicit regression guard: two contributions with the same
contributor and the same value, but different `contribution_id`s (the
ordinary shape of "two real, separate pieces of work that happen to be
worth the same amount"), both count in full. This is the correct CASE 3
outcome — the absence of dedup is not a residual bug, it is the
deliberate result of refusing to guess.

### The impossibility, proven executably

`path_046_5_replay_identity_impossibility` constructs a single
`ContributionEvent` value and shows that "WORLD A: one real event,
delivered twice" and "WORLD B: two distinct real events that happen to
match on every field" produce **byte-identical** `Vec<ContributionEvent>`
values (`assert_eq!` on the two worlds' event vectors passes trivially,
because both are built by cloning the same value). Consequently,
`allocate_contribution_pool` — or any deterministic function over
`&[ContributionEvent]` — computes the identical total contribution for
both worlds, even though the economically correct totals differ (40 for
WORLD A's true single event, 80 for WORLD B's two independent events).
No dedup function could satisfy both worlds' correct answers
simultaneously from this representation. This is FIND-018/019's
central-obstruction pattern (identical representation, opposite required
ground truth — first proven there for graph topology and identity
claims) recurring, unmodified in shape, at the allocation input
boundary.

## Proposed minimal identity contract for a future mission

Not implemented here — offered only as the smallest addition that would
move this from CASE 3 to CASE 1/2, for a future mission to evaluate and
implement deliberately:

```rust
pub struct ContributionEvent {
    pub contributor_id: String,
    pub contribution_id: String,      // remains a human-readable label
    pub event_identity: EventIdentity, // NEW: the authoritative key
    pub base_work_value: f64,
    pub factors: ContributionFactors,
}

pub struct EventIdentity {
    pub source_id: String,   // which upstream system/ingestion path minted this
    pub event_id: String,    // that source's own idempotency key for this exact occurrence
}
```

The two-part shape (source-scoped, CASE 2) is deliberately suggested
over a bare global `event_id` (CASE 1), because a single global
namespace requires every future caller/ingestion path to already
coordinate on non-colliding IDs — an assumption this repository has no
mechanism to enforce (the same issuer/authority gap FIND-019 already
found for every other bare-string identifier here). A future mission
would still need to decide, and test, the exact behavior on exact-key
replay with a *changed* payload (reject, per this mission's Part B
adversarial questions, is the safer default absent other evidence) —
that decision is explicitly not made here.

## OBSERVED

1. NaN/Infinity were accepted by production before this patch and are
   rejected after it, at four call sites, with no change to any
   previously-valid behavior (Part A).
2. No authoritative event identity exists anywhere in
   `ContributionEvent`, `distribution.rs`, `capital.rs`, or
   `stewardship.rs` (Part B inventory).
3. `capital.rs`'s `CapitalClaim` and `stewardship.rs`'s
   `StewardshipFactors`/`StewardshipWeights` share the IDENTICAL
   `value < 0.0 || value > 1.0` NaN-bypass pattern Part A fixed for
   contribution. **Not patched in this mission** — out of scope (this
   mission's authorization was scoped to "the two concrete defects
   observed in PATH-046/FIND-023," which were specifically about
   `ContributionEvent`/`allocate_contribution_pool`). Recorded here so
   it is not silently forgotten.
4. The byte-identical-representation proof
   (`path_046_5_replay_identity_impossibility`) holds regardless of pool
   size, contributor count, or which fields are inspected — it is a
   structural fact about the type, not a property of any specific test
   fixture.

## DERIVED

- **Fixing a numeric-domain defect and fixing an identity defect are
  categorically different kinds of remediation.** The finite-value fix
  required no new information — `is_finite()` is a total, well-defined
  function of the existing `f64` value, so tightening validation was
  purely a matter of adding a missing check. The replay fix would
  require NEW information this repository does not currently collect
  (an authoritative identity) — no amount of validation-tightening on
  the EXISTING fields can supply it, which is why CASE 3's correct
  action is to stop and report, not to approximate.
- **The forbidden-key list in this mission's brief is not arbitrary
  caution — every one of those keys is exactly `contribution_id`,
  `contributor_id`, or a derivative of the payload this repository
  already has**, and the impossibility proof shows precisely why each
  would fail: any key built only from the existing fields cannot
  distinguish the two worlds, because the two worlds are represented
  identically in those fields by construction.

## NOT ESTABLISHED

- Whether `capital.rs`/`stewardship.rs`'s identical non-finite gap (item
  3 above) should be patched in a follow-up mission using the same
  pattern established here. Not decided, not attempted.
- Whether the proposed `EventIdentity{source_id, event_id}` shape is the
  right one, or whether a different scoping (e.g. per-allocation-round
  rather than per-source) would serve better. Offered only as a
  starting point for a future mission's own design work.
- Whether exact-identity-with-changed-payload should be rejected or
  handled some other way — flagged as the safer default, not decided.
- Whether any real-world caller of this repository's allocation
  functions already has an external idempotency mechanism (e.g. at a
  transport layer outside this codebase) that would make CASE 3 less
  urgent in practice. Nothing in this repository answers that; this
  finding is scoped strictly to what the repository's own types and
  functions can distinguish.

## Verdict

**READY FOR CROSS-REVIEW.**

Part A (non-finite defect): FIXED, per this mission's success condition
— all six numbered requirements met (NaN rejected, +Infinity rejected,
-Infinity rejected, no successful allocation contains non-finite shares,
valid finite behavior unchanged, targeted tests pass).

Part B (replay defect): OUTCOME B (PROVEN BLOCKER), per this mission's
success condition — no authoritative identity exists, the impossibility
is proven executably, no heuristic was introduced, and a minimal
proposed identity contract is documented for a future mission to
evaluate deliberately.
