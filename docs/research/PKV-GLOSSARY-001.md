# PKV Glossary 001

Terms as already established in `docs/findings/`. No new terminology.

- **OccurrenceKey** — the canonical durable-memory key (e.g. `{namespace, event_id}`) used to claim/look up `Reserved`/`Settled` state. (FIND-026/027)
- **SourceId** — a raw, self-declared identifier on an incoming event; lexical inequality between two SourceIds proves nothing about real identity by itself. (FIND-025)
- **NamespaceId** — the canonical identity a `SourceId` resolves to via an exogenous trust contract; a distinct type, never inferred from string comparison. (FIND-025)
- **AUTHORITY** — whether a raw `source_id` string is bound to any real namespace at all; namespace identity, not string membership. (FIND-025)
- **STABILITY** — whether one real occurrence's identifier is guaranteed stable across delivery and retry within a namespace; governs same-`event_id` resolution. (FIND-025)
- **SEPARATION** — whether distinct real occurrences within a namespace are guaranteed distinct identifiers; governs different-`event_id` resolution; independent of STABILITY. (FIND-025)
- **Reserved** — the local durable claim state for an `OccurrenceKey`, recorded before the external effect/settlement executes; has its own linearization point under concurrency. (FIND-027/028)
- **Settled** — the local durable state recorded after the operation completes; can fail to be recorded independently of whether the external effect ran. (FIND-027/028)
- **Unresolved** — the honest "insufficient exogenous fact" outcome (missing AUTHORITY, STABILITY/SEPARATION, or HistoryCompleteness); never a guessed Replay/Distinct. (FIND-025/026)
- **Conflicted** — the outcome when the same identity carries incompatible payloads or disagreeing evidence; an active contradiction, not mere missing information. (FIND-025/026)
- **HistoryCompleteness** — a per-namespace attested fact (`AttestedComplete`/`Unknown`) that a negative lookup may be trusted as true absence; gates negative evidence only, never a positive record hit. (FIND-026)
- **completion memory** — provider-side durable record that an effect completed, read by recovery via `query_completion`; can itself lag the real effect. (FIND-029)
- **completion evidence** — the positive or negative signal completion memory yields; must be authoritative in both directions to be trusted — a positive hit must imply the effect happened, a miss must imply it didn't. (FIND-028/029)
- **effect operation id** — `EffectOperationId`, the canonical id passed to an external effect provider (equal to the canonical `OccurrenceKey`); names an operation only, never itself proof of completion. (FIND-028/029)
- **replay** — the `Replay` resolver outcome: the incoming event is the same real occurrence recurring, established only under AUTHORITY plus STABILITY/SEPARATION, never from key match alone. (FIND-025)
- **canonicalization** — two distinct senses: (a) topology-level, deterministic collapse of representation-only graph nodes (FIND-015); (b) identity-level, resolving a raw label to a canonical NamespaceId/OccurrenceKey before any comparison or claim (FIND-025/027). Do not conflate.
- **single-winner boundary** — an atomic, indivisible transition ensuring exactly one winner across concurrent callers; established for claim-insertion (`Reserved(K)`) specifically, not proven universal for the recovery decision itself. (FIND-027)
- **ground truth** — what actually happened (e.g. `GroundTruthEffectLog.real_effect_count`), knowable only via the test's own oracle, never via any function recovery can call. (FIND-029)
- **recovery-visible state** — the subset of durable state a real recovery function can read (e.g. `CompletionMemory.receipts`, ledger records); can be identical across worlds with different ground truth. (FIND-029)
- **negative evidence** — a lookup finding no matching record; requires an explicit completeness attestation before it may be trusted as absence, since a miss is not automatically novelty. (FIND-026/029)
- **positive evidence** — a lookup finding a matching record; must itself be authentic before it may be trusted, since a forged positive defeats recovery exactly as an incomplete negative does. (FIND-028 PATH-050-K/K2, FIND-029)

## Dangerous confusions

- Raw `SourceId` string vs. canonical `NamespaceId` — lexical inequality of two source strings is not proof of two real namespaces. (FIND-025)
- Identity key (`OccurrenceKey`/`EffectOperationId`) vs. completion evidence — an operation id names an operation; it is never itself evidence that the operation completed. (FIND-028/029)
- Missing record vs. proof of novelty — absence != novelty; a miss must be gated by an explicit completeness/authenticity attestation, never assumed true. (FIND-026/029)
- Concurrency serialization (single-winner claim boundary) vs. temporal identifiability (durable cross-call history) — one governs races at one instant, the other correct behavior varying with past state. (FIND-027 vs. FIND-026)
- STABILITY vs. SEPARATION — independent boolean properties gating different resolution directions; neither implies the other. (FIND-025)
