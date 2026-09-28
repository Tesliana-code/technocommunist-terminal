# FIND-040 — Pinned-Root Authentication Establishes Witness Origin but Not Interval Completeness

Status: OBSERVED / DERIVED / NOT ESTABLISHED
Protocol: PP-0.1-draft
Evidence: PATH-062 (`tests/pathological_cases.rs`, `path_062_*`)
Canonical prior commit: `0b3ff70`

## Hypothesis under test

H33 (predeclared, unchanged):

> Self-contained integrity or authentication metadata cannot establish the authenticity of an interval witness when an attacker can reproduce or substitute all candidate-visible trust material. An external/pinned trust root may establish that a witness was authenticated under that root, but this is conditional on the trust root itself. Even authenticated issuer origin does not by itself establish that the witness is semantically interval-complete unless the model separately grants the issuer/source that completeness property. Therefore authentication may reduce one ambiguity while relocating the remaining trust boundary rather than eliminating it.

The experiment was written to disprove H33. Each candidate was given the strongest rule its visible input allowed.

## Prior boundary

FIND-039 (PATH-061): a genuine interval-complete witness over (O, U] carries the information needed for use-time latestness. A forged witness with the same visible shape makes the candidate unsound. The trust boundary moved to (1) witness authenticity and (2) interval completeness.

## Test model

- Reuses PATH-061: `Path061World`, `Path061CurrentnessObservation`, `Path061IntervalWitness` (the witness body), `Path061UseVerdict`, `path_061_verdict_is_sound`, and the PATH-061 Candidate C latestness rule. Each PATH-062 candidate only decides whether to apply that rule. O = 100, U = 110. Integer event sequence, no wall clock.
- **Test-local primitives. None of these are cryptography.**
  - `path_062_digest`: a public, keyless FNV-1a style mix. It models detection of accidental mutation only. No collision resistance is claimed, and none is needed, because every attack recomputes the digest honestly.
  - `path_062_sign` / `path_062_verify`: a transparent mix that anyone could forge. The **only** modeled security assumption is idealized unforgeability: a tag with `key_id == k` is constructed only by whoever holds `Path062SigningKey { id: k }`. Every attacker in the tests signs only with keys it holds. Under that assumption, a successful verify means "the holder of key k produced this tag over this digest", and nothing more.
- **Oracle-only**: `Path061World`; `Path062TrustedAuthority` (which key belongs to the authority that should be trusted); `Path062RootReality` (whether the pinned root is trusted or substituted); `Path062HonestIssuer.source` (the PATH-061 read source, current or frozen at O); `Path062IntervalCompleteness` (computed by comparing the witness with the world).
- **Candidate-visible**: the observation, the witness object (body, digest, links/seal, claimed verifier, tag), U, and, for Candidates D–F, an independently supplied `pinned: Option<Path062VerifierId>` and an optional `Path062CompletenessGrant`.
- Honest issuer: `path_062_issue` signs exactly what the issuer's read source shows and claims completeness through U. A frozen source makes the claim false. The issuer is still the real origin of the bytes.

Properties kept separate: byte integrity, structural consistency, issuer authenticity (origin), root trust, semantic interval completeness, and use-time latestness.

## Candidates tested

| Candidate | Visible trust material | Rule | Tests |
|---|---|---|---|
| A — self-hashed | body + digest | digest recomputes → PATH-061 C | `path_062_a`, `path_062_b` |
| B — hash-linked | chain anchored to (O, epoch at O), per-entry links, seal over `complete_through` | chain is structurally consistent and anchor matches the observation → PATH-061 C | `path_062_c`, `path_062_d` |
| C — self-authenticated | body + digest + claimed verifier + tag | verifies under the verifier the witness carries → PATH-061 C | `path_062_e`, `path_062_f` |
| D — pinned root | C + independently supplied `pinned` | claimed verifier equals the pin and verifies under it → PATH-061 C | `path_062_g`, `path_062_h`, `path_062_i` |
| E′ — dual pinned agreement | two witnesses from two independently pinned roots | both pass D and the bodies are equal → PATH-061 C | `path_062_j` |
| F — pinned root + completeness grant | D + `Path062CompletenessGrant { verifier, through }` | grant is bound to the pin and `through >= U` → D | `path_062_k`, `path_062_l` |

## OBSERVED

1. **A detects accidental mutation.** Clearing the transitions without updating the digest gives `Unresolved`. Byte integrity is a real, separate property (`path_062_a`).
2. **A cannot tell origin.** World G is quiet and its witness is genuine. World F has epoch 4 at 105, and the forger omits it and recomputes the digest. The tuple `(observation, body, digest, U)` is identical across the two worlds. The candidate returns `CurrentLatest(3)` in both, which is sound in G and unsound in F (`path_062_b`).
3. **B detects mutation, truncation, and re-anchoring.** Changing one entry's epoch, dropping the last entry without resealing, or changing the anchor epoch each fails the structural check and gives `Unresolved` (`path_062_c`).
4. **B cannot tell origin.** The forger builds a complete alternative chain from the public anchor with the same public builder, omitting 105. It is structurally consistent and equal to world G's genuine chain. The verdicts are identical, and F is unsound. A self-consistent chain that invents epoch 4 at 106 in the quiet world is also accepted, returning `CurrentLatest(4)` where the truth is 3 (`path_062_d`).
5. **C detects tampering under a real authority's tag, but accepts a full substitution.** Replacing the body while keeping the authority's tag gives `Unresolved`. An attacker that replaces the body, digest, tag, and carried verifier with its own key passes verification. It does not violate the unforgeability assumption. The result is `CurrentLatest(3)`, which is unsound (`path_062_e`).
6. **C: the carried key does not identify the trusted authority.** In world G, key K belongs to the authority and the witness is genuine. In world F, the authority has a different key, and the attacker holds K and signs an omitting witness. The visible input, including a valid tag, is identical. The candidate returns the same verdict in both worlds, sound in G and unsound in F. Which key is trusted exists only in the oracle (`path_062_f`).
7. **D with a trusted pin.** A genuine witness gives `CurrentLatest(4)`, which is correct. Each of these gives `Unresolved`: a forgery signed with the attacker's key, a forgery that claims the pinned identity but carries the attacker's tag, and the authority's genuine tag replayed over a substituted body. A missing pin with a **genuine** witness also gives `Unresolved`, so a missing root means authentication is unavailable, not that the witness is forged (`path_062_g`).
8. **D with a substituted pin.** In world T, pin K is the authority's key and the witness is genuine. In world S, the pin was replaced with the attacker's K, and the attacker signs an omitting witness. The visible `(observation, witness, pin, U)` is identical. The verdicts are identical, T is sound, and S is unsound (`path_062_h`).
9. **Central result: D with a trusted pin still fails on an authentic but incomplete witness.** In world Q (quiet), the authority's source is current. In world L (epoch 4 at 105), the **same authority with the same key** has a source frozen at O. It honestly signs what it sees and claims completeness through U. Both witnesses verify under the genuinely trusted pin. The oracle marks both origins as genuine, Q as complete, and L as incomplete. The candidate-visible input, including the valid tag, is identical. The candidate returns `CurrentLatest(3)` in both: sound in Q, unsound in L. The world has no forger and no substituted root (`path_062_i`).
10. **E′: agreement does not supply completeness.** Two independently pinned, honest authorities that read the same lagging upstream produce authentic, agreeing witnesses. The result is `CurrentLatest(3)`, which is unsound. With current sources, the same candidate is sound (`path_062_j`).
11. **F is conditionally sound.** Across six worlds, the root is trusted and the authority's source is current, so the grant is true. The worlds cover no change, a change at O+1, a change mid-interval, a change exactly at U, two changes, and a change at U+1. F returns the correct verdict in every one. The same authentic witness with no grant, or with a grant ending at U−1, gives `Unresolved` (`path_062_k`).
12. **F's grant is load-bearing and not derived.** Suppose the grant is false because the authority's source lags. F still accepts the authentic witness and returns `CurrentLatest(3)`, which is unsound. With a substituted root and a matching grant for that root, F also accepts a forgery (`path_062_l`).

## DERIVED

- **Byte integrity and structural consistency are not origin.** Both are checkable from the witness alone, and both catch accidental or partial corruption (observations 1 and 3). A forger with the public construction function rebuilds them for any body, so neither can separate genuine from forged worlds (observations 2 and 4).
- **Self-contained authentication reduces to a key-to-authority binding that is not visible to the candidate.** When the verifier identity travels with the witness, a valid tag shows only that some key-holder signed. Which key is the trusted authority's key is oracle-only (observation 6). Idealized unforgeability does not change this.
- **A pinned root establishes "authenticated under the pinned verifier", conditional on the pin.** When the pin is trusted, D rejects every tested forgery and replay (observation 7). The assumption that makes D succeed is that the independently supplied pin is the real authority's verifier, together with idealized unforgeability. D cannot check the pin. A substituted pin gives identical visible input with a different correct verdict (observation 8). The root is not self-authenticating.
- **Origin authenticity and interval completeness are separate properties that need separate assumptions.** Observation 9 fixes the root as trusted, the origin as genuine, and the tag as valid, and still produces an identical-input pair with different verdicts. "The trusted issuer produced these bytes" holds in both worlds. "These bytes contain every authoritative transition through U" holds in only one. No function of the visible input, including a perfect authentication check, can be sound in both worlds while committing.
- **Replicated authentic attestation does not supply completeness when the attesters share a source.** This is the FIND-037 pattern appearing again at the authenticated-witness layer (observation 10).
- **Under explicit assumptions, the PATH-061 rule is sound.** Those assumptions are a trusted pin, idealized unforgeability, and a true completeness grant through U for the pinned issuer. The grant is an input that the mechanism cannot verify. When it is false, the mechanism fails (observations 11 and 12).

H33 outcome: **survived** every attempt to disprove it. No tested candidate defeated a counterexample under weaker assumptions than H33 states. Its conditional clauses were confirmed separately: the pinned root is conditional on the root (observation 8), and completeness is conditional on a grant (observations 9 and 12).

## HYPOTHESIZED

- A denser witness encoding with one entry per sequence number in (O, U] would change only the witness shape. An issuer with a frozen source would report "no change" at each sequence, so the observation 9 pair would carry over unchanged. Not tested.
- The completeness grant is itself an authority claim about the issuer's source. It may reduce to the FIND-037 history-completeness problem one level up. Not tested.

## NOT ESTABLISHED

- Any property of real digital signatures, MACs, hash functions, PKI, certificate authorities, key rotation, HSMs, transparency logs, consensus, quorum, or Byzantine fault tolerance. The test primitives are trivially forgeable. Unforgeability is an assumption the attackers obey, not a property of the code.
- Collision resistance. No attack depends on it.
- How a trust root is distributed, pinned, rotated, or recovered.
- How an issuer could establish, or a candidate could check, interval completeness of the issuer's own source.
- That authentication is useless. Under a trusted pin it separates forged-origin worlds from genuine-origin worlds (observation 7).
- Any production type, protocol, or economic-allocation wiring. `src/` is untouched.
- Novelty.

## Strongest counterexample

`path_062_i`: the candidate-visible input is identical in both worlds, and authentication under a genuinely trusted pinned root succeeds in both:

```
observation = (epoch 3 @ O=100)
witness     = { body: { from: 100, complete_through: 110, transitions: [] },
                digest, claimed_verifier: A, tag: sign(A, digest) }
pinned      = A        (genuinely the authority's verifier in both worlds)
use         = U=110
```

- Oracle world Q: no transition after 10. The authority's source is current. The witness is authentic and complete. The latest at U is 3.
- Oracle world L: epoch 4 at 105. The authority's source is frozen at O. The witness is authentic and incomplete. The latest at U is 4.

Candidate D returns `CurrentLatest(3)` in both worlds. The pair uses no forger and no root substitution, so the failure cannot be attributed to authenticity.

## Outcome classification

- A (self-hashed) and B (hash-linked): **positive for integrity and structural consistency**, **negative for origin**. Disproved by indistinguishable worlds.
- C (self-authenticated): **negative for origin**. Disproved by an indistinguishable pair in which the key-to-authority binding differs.
- D (pinned root): **conditionally positive for origin** under a trusted pin plus idealized unforgeability. **Negative for root trust**: disproved by an indistinguishable pair with a substituted pin. **Negative for interval completeness**: disproved by the authentic-but-incomplete pair.
- E′ (dual pinned agreement): **negative for completeness** when the attesters share a lagging source.
- F (pinned root + completeness grant): **CONDITIONAL positive**. Sound whenever the pin is trusted and the grant is true. The mechanism derives neither assumption.

## Stop condition / next boundary

Authentication moves the origin question onto an exogenous root. It does not touch interval completeness. Use-time latestness in the model now rests on three separate exogenous assumptions: (1) the pinned root is the real authority's verifier, (2) idealized unforgeability, and (3) the authenticated issuer's source is interval-complete through U. The next boundary is whether issuer-source completeness (assumption 3) can be made visible to the candidate without another grant, or whether it reduces to the FIND-037 completeness problem. That belongs to a separate PATH.

## Validation state

- `cargo test --test pathological_cases path_062`: 12 passed, 0 failed
- `cargo test --test pathological_cases path_061`: 8 passed, 0 failed (regression)
- `cargo check --bins`: clean
- `git diff --check`: clean
- rustfmt: not applied (no whole-file formatting of `tests/pathological_cases.rs`)
- production `src/`: untouched
- PostgreSQL / ignored tests: not run
- no commit, no push
