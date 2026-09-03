# FIND-016 — Explicit Representation Roles Reduce Ambiguity but Create a New Laundering Surface

Status: OBSERVED / HYPOTHESIZED / NOT ESTABLISHED (see split below)
Protocol: PP-0.1-draft
Evidence: PATH-039

## Hypothesis under test (H10)

Explicit representation semantics can safely authorize canonicalization of
representation-only nodes while preventing semantic-capability collapse
and resisting trivial declaration laundering.

## OBSERVED

1. **No existing type carries representation-role or issuer/authority
   semantics.** `DependencyId`, `DependencyPathId`, and `FailureDomainId`
   are bare `String`-wrapped identifiers with no such concept;
   `RelayCollapseProvenance` (PATH-038) records only WHAT was collapsed,
   never WHY it was eligible. The new types
   (`RepresentationRole`, `RepresentationClaimId`, `RepresentationClaim`,
   `RepresentationDecision`) follow this repository's existing minimal
   `XxxId(String)` + plain-struct convention and add no organizational-
   authority or manager-hierarchy concept.

2. **The contract tested (`canonicalize_declared_representation_nodes`):**
   a node collapses only if it clears every PATH-038 topology guard AND
   `evaluate_representation_claims` returns `Authorized` — a valid
   `RepresentationOnly` claim exists (non-empty `claim_id`) with no
   conflicting `SemanticCapability` claim for the same capability.
   `Unclassified` is deliberately the ABSENCE of any claim, not a third
   enum variant (`path_039_b`), and never authorizes collapse
   (conservative default). Conflicting claims produce
   `RepresentationDecision::Conflicted`, always surfaced via
   `representation_claim_conflicts`, never silently resolved
   (`path_039_k`/`p` — no last-writer-wins).

3. **Topology guards remain necessary in addition to semantic marking.**
   A branch point (`path_039_e`) or convergence point (`path_039_f`)
   marked `RepresentationOnly` still never collapses, because neither can
   satisfy the one-predecessor/one-successor shape regardless of
   authorization. A cycle member marked `RepresentationOnly`
   (`path_039_g`) still never collapses. Duplicate declarations, edge
   order, declaration order, and relabeling do not affect the outcome
   (`path_039_h/i/j/r`); a swarm of 25 irrelevant declarations about
   capabilities absent from the graph has zero effect (`path_039_m`);
   idempotence holds (`path_039_s`); every collapsed node remains
   individually traceable via provenance (`path_039_t`, though this is
   explicitly NOT claimed as full graph losslessness).

4. **Explicit semantics DO close PATH-038-N's specific gap
   (`path_039_n`).** Given a TRUTHFUL declaration, `SOURCE->X->TARGET`
   now canonicalizes differently depending on whether X is declared
   `RepresentationOnly` (collapses) or `SemanticCapability` (survives) —
   a deterministic difference topology alone could never produce.

5. **Explicit semantics do NOT resist a false declaration
   (`path_039_d`).** A structurally well-formed `RepresentationOnly`
   claim about a capability stipulated to be genuinely meaningful
   collapses it exactly as a true claim would. The function has no way
   to verify claim truth, only claim well-formedness (non-empty
   `claim_id`, no conflicting claim).

6. **Representation identity laundering is real and undetected
   (`path_039_l`)** — the mission's central "resurrection quest" concern
   confirmed: declaring BOTH halves of a stipulated single real capability
   (`X1 -> X2`) `RepresentationOnly` erases the entire boundary
   (`SOURCE->TARGET` directly), with zero conflict raised, because each
   individual declaration is internally consistent. The declarations
   themselves are the only evidence, and nothing here verifies their
   content is true.

7. **Candidate A (bare boolean) is structurally rejected, not merely
   attacked and surviving weakly (`path_039_candidate_a`).** A bare
   `HashSet<CapabilityId>` "is representation-only" model cannot even
   represent an explicit `SemanticCapability` declaration — "never
   considered" and "explicitly declared meaningful" are both just
   "absent from the set," indistinguishable. It also cannot distinguish
   a duplicate declaration from independent corroboration (both are the
   same set-membership fact). This candidate does not merely fail the
   laundering attacks; it cannot even express the states PATH-039-B/C
   require to be distinguishable in the first place.

8. **Candidate C (corroboration via distinct claim_ids, no identity)
   also fails (`path_039_candidate_c`).** Requiring >= 2 distinct
   `claim_id`s asserting `RepresentationOnly` before authorizing collapse
   is trivially satisfied by a single undisclosed actor submitting two
   claims under two different `claim_id` strings — this repository has no
   issuer-identity/authority concept to make "two claims" mean "two
   independent parties agreed," and none is invented here per mission
   constraints. Multiplicity of unauthenticated claim identities
   establishes nothing about truth.

9. **Candidate D (external semantic evidence / authenticated
   authority/attestation) was not implementable or tested in this
   mission.** It is one plausible direction for resisting the D/L
   laundering attacks, but authenticated issuer identity alone would
   establish provenance of the assertion, not necessarily its semantic
   truth. This mission neither builds nor tests it, and does not
   exhaust every possible future evidence mechanism; no claim is made
   that Candidate D works, or that it is uniquely capable of working.

10. **The deepest result: even fully TRUTHFUL, individually-correct
    declarations reproduce PATH-038-K/P's cascading erasure at an
    UNCLAIMED node (`path_039_o`).** X and Y — both genuinely,
    correctly declared `RepresentationOnly` — collapse into `source -> m`
    exactly as PATH-038 predicted; M is correctly left unclaimed
    (Unclassified, the correct choice for a real convergence point) and
    therefore never itself authorized to collapse, but its predecessor
    count still silently drops from 2 to 1 in the canonical edges purely
    from X and Y's bypass edges deduplicating onto the same ancestor.
    `shared_structural_bottlenecks` on the canonical graph reports
    nothing. This is NOT a declaration-truthfulness problem and explicit
    semantics do not touch it at all: it is a consequence of how bypass
    edges deduplicate, entirely orthogonal to any node's semantic status.

## HYPOTHESIZED

- That an authenticated issuer-identity/attestation system (Candidate D)
  could resist the D/L laundering attacks. Not designed or tested; would
  require infrastructure and/or governance policy this mission is
  explicitly scoped to avoid inventing.
- That some structural corroboration NOT reducible to "count distinct
  claim_ids" — e.g. requiring a `RepresentationOnly` claim to be
  cross-referenced against an independent, previously-established
  structural fact — could raise the cost of laundering without inventing
  an authority model. Not designed or tested.
- That a future evidence model addressing PATH-038-K/P's cascading
  erasure (a different edge-deduplication or multiplicity-aware
  representation, per FIND-015's HYPOTHESIZED section) would compose
  cleanly with this representation-claim layer. Not tested.

## NOT ESTABLISHED

- **Who is allowed to issue a representation claim.** No authority,
  role, or governance model exists or is proposed; `claim_id` is bare
  evidence identity, not a permission grant.
- **How semantic truth is externally verified.** Nothing in this mission
  verifies that a `RepresentationOnly` or `SemanticCapability` claim
  corresponds to reality; `path_039_d`/`l` demonstrate the converse.
- **Whether conflicting evidence can be resolved.** `Conflicted` is
  reported, never resolved; no resolution mechanism is proposed as
  correct or complete.
- **Whether canonicalization preserves every relevant structural fact.**
  `path_039_o` shows it does not, even under fully truthful declarations;
  `path_039_t` establishes only node-level traceability, not general
  losslessness.
- **Whether representation semantics are economically meaningful.**
  Nothing here is wired into `CriticalityObservables`,
  `measure_criticality`, `Contribution`, or any `aggregate_*`/allocation
  function, and no claim is made that any of these types should be.
- **Whether any scalar blast-radius metric becomes valid after this
  layer.** FIND-014's rejection of tested scalar reductions is
  unaffected; this mission does not revisit or rehabilitate any of
  candidates A/B/C from PATH-037.
- That representation laundering is impossible. The opposite is shown:
  `path_039_l` is a confirmed laundering example that raises no conflict
  and requires no topological anomaly.

## Strongest counterexample

`path_039_o`: two fully truthful, correctly-evidenced, individually
non-conflicting `RepresentationOnly` declarations, with the one node that
actually needed protecting (M) correctly left unclaimed by the caller,
still reproduce the exact structural erasure PATH-038 found without any
semantics at all. This shows the representation-claim layer answers a
narrower question ("is this specific node's collapse authorized by
someone") than the one canonicalization safety actually depends on ("does
this specific collapse, combined with every other collapse that will
happen in the same pass, preserve every node's structural facts") — and
no amount of correct, truthful, per-node declaration closes that gap.

## Outcome classification

**H10 REJECTED as stated; a narrower claim survives.** Explicit
representation semantics measurably reduce one specific ambiguity
(PATH-038-N: topology alone cannot distinguish a relay from a meaningful
capability) — given a truthful declaration, the two cases now
deterministically diverge. But H10's full claim is false on two
independent grounds: (a) the declaration mechanism itself is a laundering
surface — no tested candidate A/B/C verifies claim truth, and Candidate D
is an untested possible direction requiring infrastructure this mission
cannot build; and (b) even perfectly truthful
declarations do not prevent PATH-038-K/P's cascading structural erasure,
which does not depend on any node's semantic status at all. The surviving,
narrower claim is: **explicit representation roles let a caller assert
"collapse this specific node" with an inspectable, conflict-aware audit
trail, and nothing more** — not truth, not safety against cascading
erasure, not resistance to a determined false or split declaration.
