# FIND-019 — Identity Truth Requires at Least One Exogenous Trust Axiom

Status: OBSERVED / HYPOTHESIZED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-042

## Hypothesis under test (H13)

A small independently-originating evidence primitive, carrying verifiable
lineage that is not derived from the identities being evaluated, is
sufficient to break at least some identity ambiguities that topology and
self-authored claims cannot.

## Background

FIND-018 (PATH-041) proved, using the mission's own graph-isomorphism pair
(`source -> x -> m`, `source -> y -> m`, identical for both a stipulated
true-independence world and a stipulated split-identity world), that no
function of topology alone can resolve which world it is looking at.
PATH-042 asks what the smallest *non-topological* addition would take to
break that tie.

## Candidates tested

**Candidate A — claim count.** Bare `RepresentationClaim` volume.

**Candidate B — independent observer identity.** `ObservedFact { observer,
left, right, asserted_distinct }`. Requires >= 2 distinct, non-self
observers to agree `x`/`y` are distinct before certifying `Independent`;
disagreement surfaces as `Conflicted`.

**Candidate C — independent event origin (structural half).**
`EvidenceEvent { event_id, content: { subject, origin_domain, fact_kind } }`,
deduplicated by `content` — directly reusing this repository's existing
`DependencyStructuralKey`/`deduplicate_structural_dependencies` pattern
(FIND-007) at the evidence layer: a free-form `event_id` is never trusted
as identity on its own.

**Candidate D — cross-domain corroboration.** Same `EvidenceEvent` shape,
but the decision function takes an explicit
`trusted_distinct_domains: Option<(&DomainId, &DomainId)>` parameter that
the function never derives from `events` — it must be supplied by the
caller as a fact known from outside the analysis.

All four are test-local (`tests/pathological_cases.rs`); no production
code was modified, and no cryptography, PKI, or authority infrastructure
was built — the "trust axiom" is a plain, inert function parameter.

## OBSERVED

1. **Candidate A is killed at arbitrary scale**
   (`path_042_candidate_a_claim_count_inflation_does_not_certify_independence`).
   50 distinct `claim_id`s per side changes nothing: `claim_id` carries no
   issuer identity (FIND-016 observation 1, unchanged), so volume is free
   for a single actor to manufacture.

2. **Self-observation contributes zero corroboration**
   (`path_042_b`). Filtering facts where `observer == subject` before
   counting witnesses correctly reduces two "self-attesting" facts to
   zero relevant evidence.

3. **A single observer establishes a witness, not corroboration**
   (`path_042_c`). One non-self observer asserting distinctness yields
   `Insufficient`, not `Independent` — corroboration requires a second,
   independently-counted observer.

4. **Observer-label distinctness alone cannot tell a stipulated alias pair
   from a stipulated genuine pair**
   (`path_042_d_and_e_observer_label_distinctness_alone_cannot_tell_alias_from_genuine`).
   Two non-self observers agreeing produces `Independent` regardless of
   whether the test's own narrative stipulates them as secretly-one-source
   aliases or as genuinely separate parties — the DATA is identical in
   both cases. This is FIND-018's topology-level impossibility proof,
   confirmed to recur unchanged one level up at the observer-identity
   layer: Candidate B relocates the question from `CapabilityId` to
   `ObserverId` without answering it.

5. **Content-addressed event dedup defeats both duplicate serialization
   and ID-laundered replay** (`path_042_f`, `path_042_g`). A literal
   duplicate event collapses to one; the identical content replayed under
   a freshly-chosen `event_id` also collapses to one — raw `event_id`
   distinctness never counted as new evidence in the first place. A
   control case (genuinely different `fact_kind`) confirms the dedup is
   content-keyed, not blanket-suppressing (`deduped.len() == 2`). This is
   a real, narrow, positive result: Candidate C's structural half solves
   PATH-042-F/G specifically, by direct transplant of the FIND-007
   principle — but it does not, by itself, address identity truth (see 4
   and 6).

6. **Cross-domain evidence without an exogenous axiom is exactly as
   inconclusive as unverified observer distinctness**
   (`path_042_d_cross_domain_evidence_without_an_exogenous_axiom_stays_insufficient`).
   Two different domain labels, one event per subject, `trusted_distinct_domains: None`
   → `Insufficient`. Domain identity is just another self-issuable string
   absent something exogenous vouching for it.

7. **An explicit, non-derived trust axiom is the minimum addition that
   changes the answer**
   (`path_042_e_an_explicit_exogenous_trust_axiom_is_the_minimum_that_resolves_independence`).
   Given the IDENTICAL data shape as observation 6, supplying
   `Some((&domain_a, &domain_b))` — a fact the function never computes,
   only consumes — flips the verdict to `Independent`. This is H13's
   narrowest surviving claim.

8. **The axiom cannot be distinguished from a forged one**
   (`path_042_axiom_forgery_produces_false_but_internally_consistent_confidence`).
   Supplying the identical kind of axiom over two domains that are
   (per the test's own stipulation) secretly aliases of one source
   produces the exact same `Independent` verdict as a true axiom would.
   The candidate has no mechanism to check the axiom; it can only
   propagate whatever it is given, faithfully and blindly.

9. **Disagreement is surfaced, not resolved** (`path_042_i`). Two non-self
   observers giving opposite answers about the same pair yield
   `Conflicted`, mirroring `RepresentationDecision::Conflicted` from
   PATH-039.

10. **Structural corroboration and evidentiary corroboration are kept
    separate** (`path_042_j`). FIND-018's structural Candidate B certifies
    `Independent` for a rich topology with zero PATH-042-layer evidence;
    PATH-042's own candidates, given no `ObservedFact`/`EvidenceEvent` at
    all, correctly stay `Insufficient` rather than inheriting the
    structural layer's confidence.

11. **Cross-domain evidence adds a representational axis, not certainty,
    by itself** (`path_042_h`). Topology (`DependencyEdge`/`CapabilityId`)
    has no domain concept at all — it cannot even pose the question. Yet
    supplying cross-domain events without an axiom produces the identical
    `Insufficient` verdict as supplying no evidence whatsoever
    (`path_042_candidate_d_cross_domain_corroboration(&x, &y, &[], None)`).
    The new axis changes what CAN be represented; it does not, by itself,
    change what CAN be concluded.

## HYPOTHESIZED

- That requiring the trust axiom itself to be independently re-derivable
  from a SECOND, unrelated exogenous source (raising the cost of forging
  it, per observation 8) would make forgery more expensive without making
  it impossible — an infinite regress of the same problem at successively
  more expensive layers, not a resolution. Not designed or tested.
- That combining Candidate B (observer count >= 2) with Candidate D
  (domain-pair axiom) — requiring BOTH multiple non-self observers AND an
  exogenous domain axiom before certifying `Independent` — would raise
  the combined cost of an attack (an adversary must now fabricate
  multiple non-self observer identities across a trust-axiom-vouched
  domain pair) without closing the gap in observation 8. Not designed or
  tested.

## NOT ESTABLISHED

- **Whether any evidence primitive expressible within this repository's
  existing type vocabulary (or reasonable extensions of it) can supply the
  trust axiom itself, rather than requiring the caller to hand it in.**
  Every primitive inspected or built here — `RepresentationClaim`,
  `ObservedFact`, `EvidenceEvent`, `DomainId` — is a bare, self-issuable
  string with no cryptographic or organizational binding to anything
  outside the data itself (consistent with FIND-016 observation 1: no
  issuer/authority type exists in this repository). Building one is an
  explicit PATH-042 non-goal, so this remains open by mission design, not
  by oversight.
- **Whether an axiom, once supplied, could be partially verified** (e.g.
  cross-checked against a second, unrelated axiom source) rather than
  taken purely on faith. Not attempted; PATH-042 tests only the single-
  axiom case.
- **Whether observer/domain corroboration composed together (see
  HYPOTHESIZED) meaningfully raises real-world attack cost**, as opposed
  to merely adding more test assertions. No cost model exists in this
  repository to evaluate that claim, and building one is out of scope.
- **Whether PATH-042-D's stipulated "two aliases of one source" and
  PATH-042-E's stipulated "two genuine parties" are the only two worlds.**
  Only these two were tested; partial-independence topologies (three or
  more entangled observers/domains, some real, some aliased) were not
  constructed.

## Strongest counterexample

`path_042_axiom_forgery_produces_false_but_internally_consistent_confidence`:
supplying a trust axiom over `alias-domain-1`/`alias-domain-2` — which, by
the same test's own construction, are exactly PATH-042-D's stipulated
aliases-of-one-source — yields `Independent`, byte-for-byte the same
outcome as the honest axiom in PATH-042-E. This shows the "minimum
external evidence" identified by this experiment is not itself a source of
truth: it is a **hole shaped exactly like the trust the caller chooses to
put into it**. The candidate's entire value lies in refusing to manufacture
that trust on its own (observations 4, 6) and in refusing to let anything
else — claim count, event replay, duplicate serialization, self-observation
— stand in for it (observations 1, 2, 5). It adds no protection against a
caller (or an attacker positioned to influence the caller) supplying a
false axiom.

## Outcome classification

**H13 is PARTIAL, with an explicit IMPOSSIBILITY BOUNDARY component.**

The experiment identifies a primitive — an explicit, externally-supplied,
non-derived `trusted_distinct_domains` fact — that satisfies all six
success conditions as literally stated:
(1) it is not derivable from graph topology (topology has no domain
concept at all, observation 11); (2) self-derived evidence (self-
observation, observation 2; claim count, observation 1) is rejected;
(3) duplicate/replay inflation is rejected via content-addressed dedup
(observation 5); (4) it distinguishes PATH-042-E's evidence shape from
PATH-042-D's identical shape, but **only because the axiom is handed in
from outside, not because the candidate discovered anything** (observations
6-8); (5) the trust assumption is maximally explicit — a plain, named
function parameter, impossible to overlook (observations 7-8); (6)
disagreement is surfaced, never silently resolved (observation 9), and
missing evidence is never upgraded from structural plausibility to
evidentiary confirmation (observation 10).

But condition 4's satisfaction is hollow in the sense the mission's Outcome
C anticipates: the primitive does not *discover* independence, it
*propagates a given fact about* independence, and propagates a false one
exactly as confidently as a true one (observation 8, the strongest
counterexample). Every layer tested — claim → observer → event → domain —
reproduces the identical pattern first proven at the topology layer in
FIND-018: two labels at that layer are indistinguishable from two aliases
of one thing UNLESS something from outside that layer vouches for them,
and that "something from outside" is, at every layer tested here, just
another bare, unauthenticated label one level up — except at the very
last layer, where this experiment stopped relocating the problem and
instead named the requirement explicitly as a function parameter rather
than inventing another self-issuable identity type to relocate it into
again.

**Minimum extra information, beyond topology, that changed the answer:**
not observer identity, not event identity, not domain identity as such —
those only changed *what could be represented*. The one thing that changed
*what could be concluded* was a single trusted exogenous fact — supplied,
never derived — asserting that two specific already-differing labels
denote genuinely different real-world trust roots. This repository (and
this experiment, by its own non-goals) has no mechanism to originate,
verify, or protect that fact; it can only refuse to fabricate it, refuse
duplicate/replayed substitutes for it, and surface disagreement about it
honestly.
