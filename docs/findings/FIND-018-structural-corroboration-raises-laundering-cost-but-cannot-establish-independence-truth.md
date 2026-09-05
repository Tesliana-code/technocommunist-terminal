# FIND-018 — Structural Corroboration Raises Laundering Cost but Cannot Establish Independence Truth

Status: OBSERVED / HYPOTHESIZED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-041

## Hypothesis under test (H12)

A node/branch can earn "independent structural origin" status only when its
independence is supported by multiple non-circular structural/evidentiary
signals, rather than by distinct node labels alone.

## Background

FIND-017 (PATH-040) confirmed a narrow invariant — provenance-aware origin
sets preserve tested pre-collapse convergence — but explicitly could not
distinguish two genuinely independent branches from two differently-labeled
representations of the same real dependency. PATH-041 attacks that specific
gap directly.

## The central obstruction, stated first

The mission's own PATH-041-A ("true independence") and PATH-041-B
("split-label laundering") cases are given in the mission text using the
IDENTICAL minimal graph shape:

    source -> x -> m
    source -> y -> m

This is not an oversight — it is the sharpest possible statement of the
problem. Any function whose only input is this graph must produce the same
output for both cases, because the input is the same. Therefore **no
function of topology alone can simultaneously (a) certify this exact shape
as independent when it is genuinely independent, and (b) reject it when it
is a split-label launder** — the two requirements are mutually exclusive
for identical input. This is confirmed directly by
`path_041_b_bare_split_label_launder_is_not_certified_independent`, which
uses exactly this minimal shape.

The only way to satisfy PATH-041-A's requirement ("preserve independence")
is to test it against a graph that has genuinely more distinguishing
structure than the bare minimal shape — which is what
`path_041_a_true_independence_with_distinguishing_structure_is_confirmed`
does. This is not a workaround; it is itself a finding: **"genuinely
independent" and "split-label launder" are not distinguishable states of
the minimal illustrative graph — they only become distinguishable once
richer structure exists**, and even then only probabilistically (see
Strongest counterexample).

## Candidates tested

**Candidate A — distinct labels only.** Independence iff the two capability
ids differ syntactically.

**Candidate B — structural corroboration.** Canonicalizes first (via the
unmodified `canonicalize_declared_representation_nodes`, protecting
`source`, `x`, `y`, and the convergence node from collapsing themselves —
this experiment corroborates whether these specific identities are
independent, not whether they are PATH-039-collapse-eligible), then
computes, per branch root, `path_041_evidence`: transitive ancestors other
than `source` ("distinct upstream ancestry"), unioned with whatever the
branch root can reach that the convergence node cannot also reach
("private downstream continuation," i.e. genuine operational divergence
before reconvergence). A verdict of `Independent` requires **both**
branches to have their own non-empty evidence set, and those two sets to
differ from each other. Either branch having empty evidence, or the two
sets being identical, yields `Insufficient` or `LikelyDuplicate` instead.

**Candidate C — non-circular evidence corroboration.** Attempted using this
repository's only evidence primitive, `RepresentationClaim`/`claim_id`
(PATH-039). Tested via `path_041_g` and rejected without needing further
development (see OBSERVED 7) — the same root cause FIND-016 already
identified for `path_039_candidate_c` (no issuer/authority identity exists
or is invented) applies unchanged here.

All three are test-local (`tests/pathological_cases.rs`); no production
code (`canonicalize_pure_relays`, `canonicalize_declared_representation_nodes`,
`shared_structural_bottlenecks`, `RepresentationClaim`) was modified.

## OBSERVED

1. **Candidate A is trivially killed**
   (`path_041_candidate_a_distinct_labels_alone_certifies_a_split_label_launder`).
   It certifies independence for any two differently-named nodes,
   regardless of what they denote.

2. **For a pair that already converges on a shared node, immediate
   successor sets carry zero information.** By definition, if x and y both
   feed into m, `successors(x) == successors(y) == {m}` always. This is why
   Candidate B's evidence function deliberately excludes raw
   predecessor/successor sets at the branch root and instead looks at
   transitive ancestry and reach *beyond* the shared convergence.

3. **Candidate B correctly certifies genuine independence when richer
   structure exists** (`path_041_a`). x fed by `db_a` with private
   continuation `audit_x`; y fed by `db_b` with private continuation
   `audit_y`. Both branches have non-empty, differing evidence →
   `Independent`.

4. **Candidate B correctly refuses the bare split-label launder**
   (`path_041_b`) — the exact minimal shape from the mission text yields
   `LikelyDuplicate`: identical predecessors, identical reachable sets,
   zero evidence on either side.

5. **Candidate B is not fooled by cosmetic relabeling/depth padding
   masquerading as distinct ancestry** (`path_041_c`). One branch padded
   with an extra representation-only relay under an unrelated name still
   canonicalizes to the same bare neighborhood as the other branch →
   `LikelyDuplicate`, not `Independent`.

6. **Candidate B does not require distinct upstream ancestry as a
   precondition** (`path_041_d`). Two branches sharing their only ancestor
   (`source`) but each with a genuine private downstream continuation
   before reconverging are still correctly certified `Independent` — a
   naive rule demanding ancestry divergence specifically would have
   wrongly rejected this.

7. **Candidate C fails for the same reason PATH-039's Candidate C already
   failed** (`path_041_g`). Two `RepresentationClaim`s, one per node, each
   with a distinct `claim_id`, satisfy a naive ">= 2 distinct claim_ids"
   corroboration rule — but each `claim_id` here is literally derived from
   the capability it claims to corroborate. `RepresentationClaim` carries
   no issuer/authority identity (FIND-016 observation 1, unchanged), so
   nothing distinguishes this from a single actor self-attesting both
   halves of one real dependency split into x and y.

8. **Candidate B is invariant to harmless representation depth on either
   side, in both directions** (`path_041_f`). Padding the genuinely
   independent case with an extra relay leaves it `Independent`; padding
   the bare split-label case with an extra relay on only one side leaves
   it `LikelyDuplicate` — depth alone moves the verdict in neither
   direction, because Candidate B canonicalizes before computing evidence.

9. **Candidate B does not silently upgrade a bare branch paired with a
   rich one** (`path_041_h`). x has genuine distinguishing structure, y has
   none; because `Independent` requires BOTH sides to carry non-empty
   evidence, y's absence of evidence forces `Insufficient` regardless of
   x's richness. (An earlier design considered here — certifying
   independence from mere set-inequality between the two branches, without
   requiring each side's evidence to be individually non-empty — was
   rejected specifically because it fails this exact case: a bare branch
   compared against a rich one would satisfy "the sets differ" purely
   because one side is empty and the other isn't.)

10. **A distinct failure mode exists that no candidate can even attempt to
    address** (`path_041_e`). Two differently-routed representations
    stipulated to denote the same real dependency, but sharing no graph
    node at all, are invisible to every candidate tested — there is no
    pair of nodes for Candidate A, B, or C to be invoked on, because the
    graph never encodes any relationship between them. This is not a wrong
    answer; it is a question the graph cannot pose.

## HYPOTHESIZED

- That composing Candidate B with PATH-039's `RepresentationClaim` layer
  (e.g., only trusting a branch's evidence when its root also carries an
  `Authorized RepresentationOnly` claim) would raise laundering cost
  further without closing the truth gap in NOT ESTABLISHED below. Not
  designed or tested.
- That a scoring/threshold variant of Candidate B (e.g., requiring evidence
  sets above a minimum cardinality, or requiring evidence to persist across
  multiple independent snapshots of the graph over time) could raise cost
  further still. Not designed or tested; "multiple snapshots over time"
  would require a temporal/versioning concept this repository does not
  have.

## NOT ESTABLISHED

- **Whether Candidate B (or any topology-only candidate) can ever establish
  ground-truth independence**, as opposed to raising the cost of the
  cheapest attack. The central obstruction above proves it cannot in
  general: PATH-041-A/B's shared minimal graph shows topology → truth is
  not a well-defined function (identical input, stipulated-opposite
  truth). By the same logic, nothing prevents an adversary willing to
  fabricate a fake `db_a`-style extra ancestor and a fake `audit_x`-style
  side effect for BOTH halves of one real, split dependency from producing
  the exact evidence shape `path_041_a` certifies as `Independent` — the
  richer signal set raises the cost of laundering (a lazy bare split no
  longer works) but does not make fabrication impossible.
- **Whether any evidence-based (non-topological) corroboration can close
  this gap without inventing an issuer/authority concept.** `path_041_g`
  shows the one evidence primitive this repository has (`RepresentationClaim`)
  cannot, because it carries no identity independent of the capability it
  describes. No other evidence primitive was found during inspection.
- **Whether Candidate B ever falsely certifies independence for two truly
  identical/duplicate branches that happen to have accidentally divergent
  padding** (e.g., two real duplicate imports that happened to attach to
  different, unrelated pre-existing nodes as "extra ancestry" by
  coincidence rather than genuine architecture) — not constructed or
  tested here; plausible but unconfirmed.
- **Whether Candidate B is complete** (recognizes every genuinely
  independent pair). It is deliberately conservative: any pair whose
  distinguishing structure happens to be minimal (even if genuinely
  independent in reality) is reported `Insufficient`/`LikelyDuplicate`,
  never `Independent`. `path_041_b`'s graph, if it had in fact been two
  real independent capabilities represented minimally, would have been a
  false negative. This trade — never falsely certifying, sometimes failing
  to certify a real case — is a deliberate design choice (satisfies success
  condition 6: never collapses ambiguous cases into "independent"), not a
  proof of completeness in the other direction.
- **Whether corroboration is possible at all for two dependencies that
  never share a graph node** (`path_041_e`'s scenario). Not merely
  untested — shown structurally unreachable by any topological method,
  since there is no shared node to reason from.

## Strongest counterexample

Candidate B cannot be defeated by the cheap attacks tested (bare relabeling,
depth padding, pairing a bare branch with a rich one, self-referential
claims) — but it can be defeated by a more expensive one not ruled out by
any success condition: an adversary who fabricates a *complete* fake
evidence set (a plausible-looking extra ancestor plus a plausible-looking
private downstream continuation) for BOTH halves of a single real
dependency they have split into two labels. Nothing in Candidate B
distinguishes genuine architectural divergence from convincingly fabricated
divergence, because both produce identical graph structure — the same root
limitation the central obstruction states formally. Candidate B therefore
does not corroborate *truth*; it corroborates *effort* — it requires an
adversary to do strictly more work (invent consistent, non-identical
surrounding structure) than the cheapest possible attack (just pick two
names), but "more work than the cheapest attack" is not independence.

## Outcome classification

**H12 REJECTED as stated ("independence... supported by... signals" implying
established truth); a narrower claim survives (outcome B, PARTIALLY
SURVIVES).** Candidate B satisfies every success condition that can be
satisfied by a topology-only mechanism: it rejects distinct-label-only
laundering (1); it preserves a genuinely independent case once that case
has any structure beyond the bare minimal shape (2, with the explicit
caveat that the bare minimal shape itself is undecidable by any candidate,
per the central obstruction); it survives harmless representation-depth
changes in both directions (3); it resists the one circular-corroboration
attempt this repository's evidence primitives permit (4); it introduces no
external authority, manual score, or oracle (5); and it never silently
certifies an ambiguous or asymmetric case as independent (6, `path_041_h`).

What does not survive is the stronger reading of H12 that independence can
be *established* rather than merely *evidenced-more-expensively*. The
mission's own PATH-041-A/B pair is a formal proof, not a tested
counterexample requiring construction: identical graph input cannot yield
different ground-truth-tracking output. Candidate C (evidence-based
corroboration) is fully rejected — this repository's only evidence
primitive is exactly as circular as PATH-039's already-rejected Candidate C,
for the identical reason (no issuer/authority identity, and inventing one
is explicitly out of scope). PATH-041-E further shows the entire
topological approach — Candidate A, B, and C alike — is blind to any pair
of dependencies that never share a graph node, regardless of real-world
relationship.
