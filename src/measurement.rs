#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CriticalityObservables {
    pub qualified_dependency_weight: f64,
    pub blocking_paths: u32,
    pub failure_blast_radius: u32,
    pub substitutability: Substitutability,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Substitutability {
    Easy,
    Moderate,
    Difficult,
    None,
}

/// PATH-034: candidate representation of explicit OPERATIONAL substitution
/// latency -- how long removal of a capability takes to reach an effective
/// substitute -- investigated as a possible additional dimension alongside
/// `Substitutability`. See the `path_034_*` test group in
/// `tests/pathological_cases.rs` and `docs/findings/FIND-011-*.md` for what
/// this experiment established and did not establish.
///
/// INSPECTION RESULT (PATH-034 first task, before this type was added):
/// `Substitutability` is NOT a binary has-substitute predicate -- it is an
/// existing four-level categorical input (`Easy | Moderate | Difficult |
/// None`) with a deterministic numeric normalization
/// (`substitutability_score`: 0.00 / 0.33 / 0.67 / 1.00). What inspection
/// actually found is narrower than "the old representation forces a
/// collapse":
///
/// - there is no documented or code-derived mapping from evidence to one
///   of these four categories (no `Evidence -> Substitutability`
///   constructor exists anywhere in this module, matching FIND-003: manual
///   scores are not economic evidence);
/// - because that mapping is unspecified, it is likewise unspecified
///   whether Easy/Moderate/Difficult are intended to track replacement
///   time, replacement effort, functional completeness, cost, some
///   combination, or something else entirely;
/// - consequently a human assigning these categories manually COULD assign
///   different categories to a 30ms-failover case and a 3-week-rebuild
///   case -- nothing here proves the old representation necessarily forces
///   them equal, only that its derivation is unspecified either way;
/// - what PATH-034 DOES establish is that the representation does not
///   EXPLICITLY encode or derive restoration time in any form: no field,
///   no derivation function, and no PATH-027..033 graph primitive
///   (`StructuralConvergenceRegion`, `distinct_entering_branch_count`, ...)
///   reads or produces a temporal quantity, and graph topology alone
///   cannot supply one either, since real-world restoration latency is not
///   a topological property of a dependency graph.
///
/// This type also does NOT establish any graph-structural meaning for
/// `Substitutability::None` (e.g. "no structural substitute path exists in
/// the dependency graph") -- no such constructor or derivation exists in
/// this codebase. `None` remains only an existing categorical value with
/// unspecified evidence derivation, exactly like `Easy`/`Moderate`/
/// `Difficult`.
///
/// `TimeToEffectiveSubstitution` is offered as ONE candidate representation
/// of explicit temporal evidence, not as the uniquely necessary one -- a
/// richer future representation (e.g. a restoration trajectory over time)
/// could encode the same distinction and derive a scalar like this one
/// from it instead of storing it directly.
///
/// This type is NOT wired into `CriticalityObservables`, `measure_criticality`,
/// any `aggregate_*` function, or `Contribution`. It exists only to test
/// whether explicit temporal distinctions are representable at all.
///
/// `std::time::Duration` is used for the bounded case because this is a
/// deterministic protocol-level input value, not a wall-clock reading --
/// no `Instant`/`SystemTime` is involved, so no runtime-clock or timezone
/// semantics leak in, and it stores an exact seconds+nanoseconds count
/// (no floating point), so unit conversions like `from_millis`/`from_secs`
/// are exact and comparable (see `path_034_d_unit_invariance`). A bespoke
/// integer-duration newtype was considered and rejected as unnecessary
/// duplication of what `Duration` already provides deterministically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeToEffectiveSubstitution {
    /// An effective substitute is reached after this exact duration.
    /// `Duration::ZERO` is a valid, meaningful value: instantaneous /
    /// already-effective failover (PATH-034-E). It is NOT used to mean
    /// "no substitution needed" or "no substitute exists" -- those are
    /// separate concepts this type does not model.
    Bounded(std::time::Duration),
    /// On the TEMPORAL axis only: no effective substitution is ever
    /// reached, on any horizon considered. This says nothing about, and
    /// does not derive from, `Substitutability::None` or any other
    /// existing categorical value -- their evidence derivation is
    /// unspecified (see above), so no claim is made about what they mean
    /// or how they relate to this variant. `Never` simply makes
    /// "never effective" representable on this axis instead of silently
    /// folding it into `Bounded` with some arbitrarily large duration.
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CriticalityMeasurementWeights {
    pub qualified_dependency_weight: f64,
    pub blocking_paths: f64,
    pub failure_blast_radius: f64,
    pub substitutability: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CriticalityMeasurement {
    pub normalized_downstream_dependencies: f64,
    pub normalized_blocking_paths: f64,
    pub normalized_failure_blast_radius: f64,
    pub normalized_substitutability: f64,
    pub score: f64,
}

impl CriticalityMeasurementWeights {
    pub fn validate(&self) -> Result<(), &'static str> {
        let values = [
            self.qualified_dependency_weight,
            self.blocking_paths,
            self.failure_blast_radius,
            self.substitutability,
        ];

        if values.iter().any(|v| *v < 0.0 || *v > 1.0) {
            return Err("criticality measurement weights must be between 0.0 and 1.0");
        }

        if (values.iter().sum::<f64>() - 1.0).abs() > 1e-9 {
            return Err("criticality measurement weights must sum to exactly 1.0");
        }

        Ok(())
    }
}

fn saturating_count(value: u32, saturation_point: u32) -> f64 {
    if saturation_point == 0 {
        return 0.0;
    }

    (value as f64 / saturation_point as f64).min(1.0)
}

fn substitutability_score(value: Substitutability) -> f64 {
    match value {
        Substitutability::Easy => 0.0,
        Substitutability::Moderate => 0.33,
        Substitutability::Difficult => 0.67,
        Substitutability::None => 1.0,
    }
}

pub fn measure_criticality(
    observables: CriticalityObservables,
    weights: CriticalityMeasurementWeights,
) -> Result<CriticalityMeasurement, &'static str> {
    weights.validate()?;

    // PP-0.1 experimental saturation points.
    // These are measurement hypotheses, not settled policy.
    if !(0.0..=1.0).contains(&observables.qualified_dependency_weight) {
        return Err("qualified dependency weight must be between 0.0 and 1.0");
    }

    let dependencies = observables.qualified_dependency_weight;

    let blocking = saturating_count(observables.blocking_paths, 5);

    let blast_radius = saturating_count(observables.failure_blast_radius, 5);

    let substitutability = substitutability_score(observables.substitutability);

    let score = dependencies * weights.qualified_dependency_weight
        + blocking * weights.blocking_paths
        + blast_radius * weights.failure_blast_radius
        + substitutability * weights.substitutability;

    Ok(CriticalityMeasurement {
        normalized_downstream_dependencies: dependencies,
        normalized_blocking_paths: blocking,
        normalized_failure_blast_radius: blast_radius,
        normalized_substitutability: substitutability,
        score,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weights() -> CriticalityMeasurementWeights {
        CriticalityMeasurementWeights {
            qualified_dependency_weight: 0.30,
            blocking_paths: 0.30,
            failure_blast_radius: 0.25,
            substitutability: 0.15,
        }
    }

    #[test]
    fn measurement_weights_sum_to_one() {
        assert!(weights().validate().is_ok());
    }

    #[test]
    fn zero_observables_produce_zero_criticality() {
        let result = measure_criticality(
            CriticalityObservables {
                qualified_dependency_weight: 0.0,
                blocking_paths: 0,
                failure_blast_radius: 0,
                substitutability: Substitutability::Easy,
            },
            weights(),
        )
        .unwrap();

        assert_eq!(result.score, 0.0);
    }

    #[test]
    fn maximum_observables_produce_maximum_criticality() {
        let result = measure_criticality(
            CriticalityObservables {
                qualified_dependency_weight: 1.0,
                blocking_paths: 5,
                failure_blast_radius: 5,
                substitutability: Substitutability::None,
            },
            weights(),
        )
        .unwrap();

        assert!((result.score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn counts_saturate_instead_of_exceeding_one() {
        let result = measure_criticality(
            CriticalityObservables {
                qualified_dependency_weight: 1.0,
                blocking_paths: 10_000,
                failure_blast_radius: 10_000,
                substitutability: Substitutability::None,
            },
            weights(),
        )
        .unwrap();

        assert!((result.score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn stronger_observables_produce_higher_criticality() {
        let low = measure_criticality(
            CriticalityObservables {
                qualified_dependency_weight: 0.2,
                blocking_paths: 1,
                failure_blast_radius: 1,
                substitutability: Substitutability::Easy,
            },
            weights(),
        )
        .unwrap();

        let high = measure_criticality(
            CriticalityObservables {
                qualified_dependency_weight: 0.8,
                blocking_paths: 4,
                failure_blast_radius: 4,
                substitutability: Substitutability::Difficult,
            },
            weights(),
        )
        .unwrap();

        assert!(high.score > low.score);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DependencyEvidence {
    pub blocking_relevance: f64,
    pub failure_propagation: f64,
    pub operational_reliance: f64,
    pub business_path_relevance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DependencyQualificationWeights {
    pub blocking_relevance: f64,
    pub failure_propagation: f64,
    pub operational_reliance: f64,
    pub business_path_relevance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QualifiedDependency {
    pub blocking_relevance: f64,
    pub failure_propagation: f64,
    pub operational_reliance: f64,
    pub business_path_relevance: f64,
    pub score: f64,
}

impl DependencyEvidence {
    pub fn validate(&self) -> Result<(), &'static str> {
        let values = [
            self.blocking_relevance,
            self.failure_propagation,
            self.operational_reliance,
            self.business_path_relevance,
        ];

        if values.iter().any(|v| *v < 0.0 || *v > 1.0) {
            return Err("dependency evidence values must be between 0.0 and 1.0");
        }

        Ok(())
    }
}

impl DependencyQualificationWeights {
    pub fn validate(&self) -> Result<(), &'static str> {
        let values = [
            self.blocking_relevance,
            self.failure_propagation,
            self.operational_reliance,
            self.business_path_relevance,
        ];

        if values.iter().any(|v| *v < 0.0 || *v > 1.0) {
            return Err("dependency qualification weights must be between 0.0 and 1.0");
        }

        if (values.iter().sum::<f64>() - 1.0).abs() > 1e-9 {
            return Err("dependency qualification weights must sum to exactly 1.0");
        }

        Ok(())
    }
}

pub fn qualify_dependency(
    evidence: DependencyEvidence,
    weights: DependencyQualificationWeights,
) -> Result<QualifiedDependency, &'static str> {
    evidence.validate()?;
    weights.validate()?;

    let score = evidence.blocking_relevance * weights.blocking_relevance
        + evidence.failure_propagation * weights.failure_propagation
        + evidence.operational_reliance * weights.operational_reliance
        + evidence.business_path_relevance * weights.business_path_relevance;

    Ok(QualifiedDependency {
        blocking_relevance: evidence.blocking_relevance,
        failure_propagation: evidence.failure_propagation,
        operational_reliance: evidence.operational_reliance,
        business_path_relevance: evidence.business_path_relevance,
        score,
    })
}

pub fn aggregate_dependency_weight(dependencies: &[QualifiedDependency]) -> f64 {
    if dependencies.is_empty() {
        return 0.0;
    }

    // Experimental V0.1 aggregation:
    // complementary probability-style accumulation.
    //
    // Many weak dependencies cannot linearly explode past 1.0,
    // while several strong independent dependencies can accumulate.
    //
    // This is a measurement hypothesis and must be attacked by
    // pathological tests before becoming protocol policy.
    1.0 - dependencies.iter().fold(1.0, |remaining, dependency| {
        remaining * (1.0 - dependency.score)
    })
}

#[cfg(test)]
mod dependency_measurement_tests {
    use super::*;

    fn weights() -> DependencyQualificationWeights {
        DependencyQualificationWeights {
            blocking_relevance: 0.30,
            failure_propagation: 0.30,
            operational_reliance: 0.25,
            business_path_relevance: 0.15,
        }
    }

    #[test]
    fn irrelevant_dependency_has_zero_weight() {
        let dependency = qualify_dependency(
            DependencyEvidence {
                blocking_relevance: 0.0,
                failure_propagation: 0.0,
                operational_reliance: 0.0,
                business_path_relevance: 0.0,
            },
            weights(),
        )
        .unwrap();

        assert_eq!(dependency.score, 0.0);
    }

    #[test]
    fn fully_critical_dependency_has_full_weight() {
        let dependency = qualify_dependency(
            DependencyEvidence {
                blocking_relevance: 1.0,
                failure_propagation: 1.0,
                operational_reliance: 1.0,
                business_path_relevance: 1.0,
            },
            weights(),
        )
        .unwrap();

        assert!((dependency.score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn stronger_dependency_evidence_produces_higher_weight() {
        let weak = qualify_dependency(
            DependencyEvidence {
                blocking_relevance: 0.1,
                failure_propagation: 0.1,
                operational_reliance: 0.1,
                business_path_relevance: 0.0,
            },
            weights(),
        )
        .unwrap();

        let strong = qualify_dependency(
            DependencyEvidence {
                blocking_relevance: 0.9,
                failure_propagation: 0.8,
                operational_reliance: 0.9,
                business_path_relevance: 0.8,
            },
            weights(),
        )
        .unwrap();

        assert!(strong.score > weak.score);
    }

    #[test]
    fn aggregate_dependency_weight_never_exceeds_one() {
        let dependency = qualify_dependency(
            DependencyEvidence {
                blocking_relevance: 0.9,
                failure_propagation: 0.9,
                operational_reliance: 0.9,
                business_path_relevance: 0.9,
            },
            weights(),
        )
        .unwrap();

        let dependencies = vec![dependency; 100];

        let aggregate = aggregate_dependency_weight(&dependencies);

        assert!(aggregate <= 1.0);
        assert!(aggregate >= 0.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyAggregationModel {
    Complementary,
    Thresholded,
    RankedDecay,
}

pub fn aggregate_dependency_weight_with_model(
    dependencies: &[QualifiedDependency],
    model: DependencyAggregationModel,
) -> f64 {
    match model {
        DependencyAggregationModel::Complementary => aggregate_dependency_weight(dependencies),

        DependencyAggregationModel::Thresholded => {
            let threshold = 0.20;

            let filtered: Vec<_> = dependencies
                .iter()
                .copied()
                .filter(|dependency| dependency.score >= threshold)
                .collect();

            if filtered.is_empty() {
                return 0.0;
            }

            1.0 - filtered.iter().fold(1.0, |remaining, dependency| {
                remaining * (1.0 - dependency.score)
            })
        }

        DependencyAggregationModel::RankedDecay => {
            if dependencies.is_empty() {
                return 0.0;
            }

            let mut scores: Vec<f64> = dependencies
                .iter()
                .map(|dependency| dependency.score)
                .collect();

            scores.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

            let mut total = 0.0;

            for (index, score) in scores.iter().enumerate() {
                let rank_decay = 0.5_f64.powi(index as i32);

                total += score * rank_decay;
            }

            total.min(1.0)
        }
    }
}

#[cfg(test)]
mod aggregation_tournament_tests {
    use super::*;

    fn weights() -> DependencyQualificationWeights {
        DependencyQualificationWeights {
            blocking_relevance: 0.30,
            failure_propagation: 0.30,
            operational_reliance: 0.25,
            business_path_relevance: 0.15,
        }
    }

    fn dependency(score: f64) -> QualifiedDependency {
        qualify_dependency(
            DependencyEvidence {
                blocking_relevance: score,
                failure_propagation: score,
                operational_reliance: score,
                business_path_relevance: score,
            },
            weights(),
        )
        .unwrap()
    }

    #[test]
    fn tournament_exposes_candidate_behavior() {
        let irrelevant = vec![dependency(0.0); 10_000];
        let weak = vec![dependency(0.01); 1_000];
        let strong = vec![dependency(0.90); 2];

        let models = [
            DependencyAggregationModel::Complementary,
            DependencyAggregationModel::Thresholded,
            DependencyAggregationModel::RankedDecay,
        ];

        for model in models {
            let irrelevant_score = aggregate_dependency_weight_with_model(&irrelevant, model);

            let weak_score = aggregate_dependency_weight_with_model(&weak, model);

            let strong_score = aggregate_dependency_weight_with_model(&strong, model);

            println!(
                "MODEL {:?} | irrelevant {:.6} | weak swarm {:.6} | two strong {:.6}",
                model, irrelevant_score, weak_score, strong_score
            );

            let irrelevant_ok = irrelevant_score.abs() < 1e-9;
            let weak_swarm_ok = strong_score > weak_score;
            let bounded_ok = strong_score <= 1.0;

            println!(
                "  verdict | irrelevant={} | weak-swarm={} | bounded={}",
                if irrelevant_ok { "PASS" } else { "FAIL" },
                if weak_swarm_ok { "PASS" } else { "FAIL" },
                if bounded_ok { "PASS" } else { "FAIL" },
            );
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DependencyId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CapabilityId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DependencyEdge {
    pub source: CapabilityId,
    pub target: CapabilityId,
}

impl DependencyEdge {
    pub fn new(source: &str, target: &str) -> Self {
        Self {
            source: CapabilityId(source.to_string()),
            target: CapabilityId(target.to_string()),
        }
    }
}

/// Graph-derived structural bottlenecks reachable from `source`.
///
/// PATH-026 showed that identifier metadata (`DependencyPathId`,
/// `FailureDomainId`) is not authoritative evidence of independence: the
/// same underlying bottleneck could be relabeled into many identifiers to
/// manufacture apparent independence. `DependencyEdge` deliberately carries
/// no such metadata, so this function can only ever see topology.
///
/// A node is a shared structural bottleneck relative to `source` when it is
/// reachable from at least two of `source`'s *distinct* immediate
/// out-edges (its structural branches). This is deliberately not "count
/// incoming edges to the node": a node can have many incoming edges from an
/// unrelated part of the graph, or from within a single branch, without
/// ever being fed by two independent branches of `source`.
///
/// Ambiguity (see PATH-027 research notes): if two branches reconverge at
/// an intermediate node X before continuing on to D, this implementation
/// reports *both* X and D as shared, because D remains reachable from both
/// of source's original branches. An alternative policy would report only
/// the first convergence point (X) and treat everything downstream of it as
/// no longer attributable to "two branches" (since from X onward there is
/// structurally only one path forward). This implementation takes the
/// "downstream reachability" reading, not the "first convergence point"
/// reading, and callers relying on the distinction should treat this as an
/// open protocol question rather than settled semantics.
/// Structural adjacency, deduped so a repeated edge cannot manufacture an
/// additional branch or inflate a convergence count. Shared by every
/// PATH-027/028/029/030 primitive below so their traversal logic does not
/// drift out of sync with each other.
fn build_structural_adjacency(
    edges: &[DependencyEdge],
) -> std::collections::HashMap<CapabilityId, Vec<CapabilityId>> {
    use std::collections::{HashMap, HashSet};

    let mut adjacency: HashMap<CapabilityId, Vec<CapabilityId>> = HashMap::new();
    let mut seen_edges: HashSet<(CapabilityId, CapabilityId)> = HashSet::new();

    for edge in edges {
        let key = (edge.source.clone(), edge.target.clone());
        if seen_edges.insert(key) {
            adjacency
                .entry(edge.source.clone())
                .or_default()
                .push(edge.target.clone());
        }
    }

    adjacency
}

/// `source`'s distinct immediate structural branches (raw, unnormalized --
/// see [`effective_structural_branches`] for the subsumption-aware
/// alternative). A self-loop back to the source is not a downstream
/// branch.
fn immediate_branch_roots(
    source: &CapabilityId,
    adjacency: &std::collections::HashMap<CapabilityId, Vec<CapabilityId>>,
) -> Vec<CapabilityId> {
    use std::collections::HashSet;

    let mut branch_roots: Vec<CapabilityId> = Vec::new();
    let mut seen_roots: HashSet<CapabilityId> = HashSet::new();
    if let Some(children) = adjacency.get(source) {
        for child in children {
            if child != source && seen_roots.insert(child.clone()) {
                branch_roots.push(child.clone());
            }
        }
    }
    branch_roots
}

/// Core of `shared_structural_bottlenecks`, generalized (PATH-030) to
/// accept an explicit branch-root set instead of always deriving it from
/// `source`'s raw immediate edges. This is what lets
/// `shared_structural_bottlenecks_from_effective_branches` reuse the exact
/// same convergence-counting traversal over a subsumption-normalized
/// branch set, instead of duplicating it.
fn shared_bottlenecks_over_branches(
    source: &CapabilityId,
    adjacency: &std::collections::HashMap<CapabilityId, Vec<CapabilityId>>,
    branch_roots: &[CapabilityId],
) -> Vec<CapabilityId> {
    use std::collections::{HashMap, HashSet, VecDeque};

    // A single branch cannot converge with anything.
    if branch_roots.len() < 2 {
        return Vec::new();
    }

    // For each branch, compute its full downstream reachable set
    // (including the branch root itself) with a bounded, visited-set BFS.
    // The per-branch visited set is what makes this cycle-safe: a cycle
    // just stops re-expanding a node it has already reached, rather than
    // looping forever or inflating the convergence count.
    //
    // A cycle that leads back to `source` is not expanded further: once a
    // branch's traversal reaches the source again, continuing on from
    // there would walk back down *other* branches too (they all start at
    // source), silently merging branches that are not actually connected
    // to each other except through the shared origin. Treating source as a
    // dead end keeps branches isolated from one another, which is the
    // property this function is supposed to be measuring.
    let mut convergence_count: HashMap<CapabilityId, usize> = HashMap::new();

    for root in branch_roots {
        let mut visited: HashSet<CapabilityId> = HashSet::new();
        let mut queue: VecDeque<CapabilityId> = VecDeque::new();
        visited.insert(root.clone());
        queue.push_back(root.clone());

        while let Some(node) = queue.pop_front() {
            if node == *source {
                continue;
            }
            if let Some(children) = adjacency.get(&node) {
                for child in children {
                    if visited.insert(child.clone()) {
                        queue.push_back(child.clone());
                    }
                }
            }
        }

        for node in visited {
            *convergence_count.entry(node).or_insert(0) += 1;
        }
    }

    let mut bottlenecks: Vec<CapabilityId> = convergence_count
        .into_iter()
        .filter(|(node, count)| *count >= 2 && node != source)
        .map(|(node, _)| node)
        .collect();

    bottlenecks.sort_by(|a, b| a.0.cmp(&b.0));
    bottlenecks
}

pub fn shared_structural_bottlenecks(
    source: &CapabilityId,
    edges: &[DependencyEdge],
) -> Vec<CapabilityId> {
    let adjacency = build_structural_adjacency(edges);
    let branch_roots = immediate_branch_roots(source, &adjacency);
    shared_bottlenecks_over_branches(source, &adjacency, &branch_roots)
}

/// PATH-030: `shared_structural_bottlenecks` computed over
/// `effective_structural_branches` instead of `source`'s raw immediate
/// edges. See the PATH-030 research notes (module-level doc on
/// `effective_structural_branches`, and FIND-008) for when this differs
/// from the raw version and why.
pub fn shared_structural_bottlenecks_from_effective_branches(
    source: &CapabilityId,
    edges: &[DependencyEdge],
) -> Vec<CapabilityId> {
    let adjacency = build_structural_adjacency(edges);
    let effective_branches = effective_structural_branches(source, edges);
    shared_bottlenecks_over_branches(source, &adjacency, &effective_branches)
}

/// Candidate FIRST-CONVERGENCE semantics for structural independence loss
/// (PATH-028).
///
/// `shared_structural_bottlenecks` reports every downstream node reachable
/// from two or more of `source`'s branches. PATH-028 investigates whether
/// that is the right economic signal, or whether only the *first* point
/// where branches converge represents a genuine structural loss of
/// independence -- with everything downstream of that point being one
/// continuation of the same loss, not additional independent losses.
///
/// A node is a first structural convergence when:
/// 1. it is a shared structural bottleneck (per
///    [`shared_structural_bottlenecks`]), AND
/// 2. no *other* shared structural bottleneck can reach it.
///
/// Condition 2 is what strips downstream continuation out of the result:
/// for `A->B->X->D->E->F, A->C->X`, D/E/F are shared bottlenecks, but X can
/// reach all three of them, so they are excluded -- only X remains.
///
/// KNOWN AMBIGUITY: if the shared bottlenecks themselves form a cycle
/// (mutual reachability between two or more of them), condition 2 makes
/// every member of that cycle disqualify every other member, and this
/// function reports *none* of them as "first" even though a real
/// structural convergence occurred just upstream of the cycle. See
/// `path_028_f_cyclic_shared_nodes_make_first_convergence_ill_defined` for
/// the smallest known counterexample. This is reported as an open
/// semantic gap, not silently resolved.
/// Core of `first_structural_convergences`: given an already-computed
/// shared-bottleneck set (from either the raw or the effective-branch
/// version of `shared_structural_bottlenecks`), strip out any node that
/// another member of that same set can reach.
fn first_convergences_over_shared(
    adjacency: &std::collections::HashMap<CapabilityId, Vec<CapabilityId>>,
    shared: std::collections::HashSet<CapabilityId>,
) -> Vec<CapabilityId> {
    use std::collections::{HashSet, VecDeque};

    if shared.len() < 2 {
        let mut trivial: Vec<CapabilityId> = shared.into_iter().collect();
        trivial.sort_by(|a, b| a.0.cmp(&b.0));
        return trivial;
    }

    let mut first: Vec<CapabilityId> = Vec::new();

    for candidate in &shared {
        let mut reached_by_other_shared = false;

        for other in &shared {
            if other == candidate {
                continue;
            }

            // Bounded, cycle-safe reachability check: can `other` reach
            // `candidate` at all?
            let mut visited: HashSet<CapabilityId> = HashSet::new();
            let mut queue: VecDeque<CapabilityId> = VecDeque::new();
            visited.insert(other.clone());
            queue.push_back(other.clone());
            let mut found = false;

            while let Some(node) = queue.pop_front() {
                if node == *candidate {
                    found = true;
                    break;
                }
                if let Some(children) = adjacency.get(&node) {
                    for child in children {
                        if visited.insert(child.clone()) {
                            queue.push_back(child.clone());
                        }
                    }
                }
            }

            if found {
                reached_by_other_shared = true;
                break;
            }
        }

        if !reached_by_other_shared {
            first.push(candidate.clone());
        }
    }

    first.sort_by(|a, b| a.0.cmp(&b.0));
    first
}

pub fn first_structural_convergences(
    source: &CapabilityId,
    edges: &[DependencyEdge],
) -> Vec<CapabilityId> {
    let adjacency = build_structural_adjacency(edges);
    let shared = shared_structural_bottlenecks(source, edges)
        .into_iter()
        .collect();
    first_convergences_over_shared(&adjacency, shared)
}

/// PATH-030: `first_structural_convergences` computed over
/// `shared_structural_bottlenecks_from_effective_branches` instead of the
/// raw shared-bottleneck set. See PATH-030 research notes / FIND-008.
pub fn first_structural_convergences_from_effective_branches(
    source: &CapabilityId,
    edges: &[DependencyEdge],
) -> Vec<CapabilityId> {
    let adjacency = build_structural_adjacency(edges);
    let shared = shared_structural_bottlenecks_from_effective_branches(source, edges)
        .into_iter()
        .collect();
    first_convergences_over_shared(&adjacency, shared)
}

/// One branch's structural entry point into a convergence region: `branch`
/// reaches the region, and the first region member it reaches from outside
/// the region is `member`.
///
/// PATH-032 correction: PATH-031's original `FIND-009` claimed that
/// `A->B->X, A->C->X, X<->Y` (both branches enter the cycle at X) and
/// `A->B->X, A->C->Y, X<->Y` (branches enter at *different* cycle members)
/// were "topologically indistinguishable". That was wrong -- the two
/// graphs have different edges (`C->X` vs `C->Y`) and were never the same
/// graph. What was actually true is narrower: `StructuralConvergenceRegion`
/// as PATH-031 first defined it (bare `members`) collapsed both graphs to
/// the identical region `{X, Y}`, because SCC *membership* does not care
/// which member a branch entered through. `entries` restores exactly that
/// distinction: `{B,X},{C,X}` vs `{B,X},{C,Y}` -- self-attesting through
/// ordinary topology, no timestamps or causal metadata required.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralConvergenceEntry {
    pub branch: CapabilityId,
    pub member: CapabilityId,
}

/// A group of shared structural bottlenecks that are mutually reachable
/// (a strongly connected component of the induced subgraph over the
/// shared-bottleneck set). PATH-031 introduces this instead of forcing a
/// cyclic convergence back into a single `CapabilityId`: on
/// `A->B->X, A->C->X, X<->Y`, both X and Y are equally part of the same
/// structural loss of independence, and reporting only a canonically
/// chosen one (e.g. the lexicographically smallest) would silently drop
/// real provenance -- which capability actually participates in the
/// convergence -- for no reason other than making the return type match
/// `Vec<CapabilityId>`. `members` is sorted for determinism and always has
/// at least one element.
///
/// PATH-032 adds `entries`: which branch (from whichever branch set the
/// caller composed with -- raw or effective) enters this region, and
/// through which specific member. See `StructuralConvergenceEntry` and
/// `compute_region_entries`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralConvergenceRegion {
    pub members: Vec<CapabilityId>,
    pub entries: Vec<StructuralConvergenceEntry>,
}

/// PATH-032: for one already-selected first-convergence region, determine
/// which branch roots enter it and through which member.
///
/// "Branch" here is deliberately whatever `branch_roots` the caller
/// passes in -- raw immediate branches, or (preferred when composing with
/// PATH-030) `effective_structural_branches`. A raw, subsumed branch root
/// can manufacture a spurious entry that disappears once the redundant
/// branch is excluded; see `path_032_g_redundant_branch_entry_disappears_
/// under_effective_composition`.
///
/// A branch's entry member is deliberately recorded as (branch root,
/// region member) and nothing finer-grained (no intermediate relay nodes
/// the branch's private path happens to pass through on the way in). That
/// is a chosen level of abstraction for this summary type, not a loss:
/// the raw `edges` a caller passed in are always still available if a
/// finer-grained path needs to be recovered later.
fn compute_region_entries(
    source: &CapabilityId,
    adjacency: &std::collections::HashMap<CapabilityId, Vec<CapabilityId>>,
    region_members: &std::collections::HashSet<CapabilityId>,
    branch_roots: &[CapabilityId],
) -> Vec<StructuralConvergenceEntry> {
    use std::collections::{HashSet, VecDeque};

    let mut seen: HashSet<(CapabilityId, CapabilityId)> = HashSet::new();

    for root in branch_roots {
        // A branch root that is itself a member of the region (e.g. a raw
        // branch root absorbed into a shared SCC) enters it trivially, at
        // itself, and does not get traversed outward for entry discovery
        // -- there is no "outside" territory to search from a starting
        // point that is already inside the region.
        if region_members.contains(root) {
            seen.insert((root.clone(), root.clone()));
            continue;
        }

        // Walk only this branch's territory OUTSIDE the region. Source is
        // a dead end (as elsewhere, so one branch's traversal cannot leak
        // into another's); a region member is ALSO a dead end here -- the
        // moment an edge lands on a region member, that member is
        // recorded as an entry and the walk does not continue past it.
        // Without this boundary, traversal could pass through one member
        // into private downstream territory and later rediscover a
        // *different* region member from there, manufacturing a second
        // "entry" that was never actually reached from outside the
        // region.
        let mut visited: HashSet<CapabilityId> = HashSet::new();
        let mut queue: VecDeque<CapabilityId> = VecDeque::new();
        visited.insert(root.clone());
        queue.push_back(root.clone());

        while let Some(node) = queue.pop_front() {
            if node == *source {
                continue;
            }
            if let Some(children) = adjacency.get(&node) {
                for child in children {
                    if region_members.contains(child) {
                        seen.insert((root.clone(), child.clone()));
                        continue;
                    }
                    if visited.insert(child.clone()) {
                        queue.push_back(child.clone());
                    }
                }
            }
        }
    }

    let mut entries: Vec<StructuralConvergenceEntry> = seen
        .into_iter()
        .map(|(branch, member)| StructuralConvergenceEntry { branch, member })
        .collect();

    entries.sort_by(|a, b| a.branch.0.cmp(&b.branch.0).then(a.member.0.cmp(&b.member.0)));
    entries
}

/// Core of the PATH-031 cycle-aware frontier: given an already-computed
/// shared-bottleneck set, group it into strongly connected components of
/// its *induced subgraph* (only direct edges between two shared nodes),
/// condense those components into a DAG, and return the components with
/// no incoming edge from another component.
///
/// ## Why the induced subgraph, and why that is not an approximation here
///
/// "Induced subgraph over S" (direct S-to-S edges only) could, in
/// general, disagree with "full-graph reachability restricted to pairs in
/// S" -- a path from `u` to `v` that detours through some node `p` not in
/// S would count for the latter but not the former. For an *arbitrary*
/// node set this would be a real modeling choice with a real trade-off.
///
/// It is not a choice here, because `S` (the shared-bottleneck set) is
/// provably **downward-closed under reachability**: `shared_bottlenecks_
/// over_branches` computes each branch's reachable set with a BFS that,
/// once it visits a non-source node, always expands all of that node's
/// children into the same set. So if `u` is in branch `i`'s reachable set
/// and there is an edge `u -> v` (with `v != source`), `v` is
/// automatically in branch `i`'s reachable set too. Consequently, if
/// `u ∈ S` (reachable from >= 2 branches) and `v` is reachable from `u` at
/// all, `v` is reachable from those same >= 2 branches, so `v ∈ S` as
/// well. Every node on *any* path between two members of `S` is therefore
/// itself a member of `S` -- there is no detour through a non-shared node
/// to miss. Induced-subgraph SCCs and full-graph-reachability SCCs
/// restricted to `S` are the same partition for this specific `S`. (This
/// would not hold for an arbitrary caller-supplied node set; it holds
/// here because of how `S` is constructed.)
///
/// Because condensing the SCCs of *any* directed graph always yields a
/// DAG, "does another component reach this one" can never itself loop
/// back into the PATH-028-f-style mutual-disqualification bug -- the
/// componenet-level ordering is well-defined by construction, not merely
/// tested-and-found-to-work.
fn cycle_aware_first_convergences_over_shared(
    source: &CapabilityId,
    adjacency: &std::collections::HashMap<CapabilityId, Vec<CapabilityId>>,
    shared: std::collections::HashSet<CapabilityId>,
    branch_roots: &[CapabilityId],
) -> Vec<StructuralConvergenceRegion> {
    use std::collections::{HashMap, HashSet};

    if shared.is_empty() {
        return Vec::new();
    }

    let mut s_list: Vec<CapabilityId> = shared.into_iter().collect();
    s_list.sort_by(|a, b| a.0.cmp(&b.0));

    if s_list.len() == 1 {
        let member_set: HashSet<CapabilityId> = s_list.iter().cloned().collect();
        let entries = compute_region_entries(source, adjacency, &member_set, branch_roots);
        return vec![StructuralConvergenceRegion {
            members: s_list,
            entries,
        }];
    }

    let n = s_list.len();
    let mut index: HashMap<CapabilityId, usize> = HashMap::new();
    for (i, node) in s_list.iter().enumerate() {
        index.insert(node.clone(), i);
    }

    let mut induced = vec![vec![false; n]; n];
    for i in 0..n {
        if let Some(children) = adjacency.get(&s_list[i]) {
            for child in children {
                if let Some(&j) = index.get(child) {
                    induced[i][j] = true;
                }
            }
        }
    }

    let mut reach = induced.clone();
    for k in 0..n {
        for i in 0..n {
            if !reach[i][k] {
                continue;
            }
            for j in 0..n {
                if reach[k][j] {
                    reach[i][j] = true;
                }
            }
        }
    }

    // Group into SCCs. Class id = index of the first (alphabetically
    // smallest, since s_list is pre-sorted) member encountered.
    let mut class_of: Vec<Option<usize>> = vec![None; n];
    for i in 0..n {
        if class_of[i].is_some() {
            continue;
        }
        class_of[i] = Some(i);
        for j in 0..n {
            if j != i && reach[i][j] && reach[j][i] {
                class_of[j] = Some(i);
            }
        }
    }

    let mut members_by_class: HashMap<usize, Vec<usize>> = HashMap::new();
    for (idx, class) in class_of.iter().enumerate() {
        members_by_class.entry(class.unwrap()).or_default().push(idx);
    }

    let class_ids: Vec<usize> = members_by_class.keys().copied().collect();
    let mut regions: Vec<StructuralConvergenceRegion> = Vec::new();

    for &c in &class_ids {
        let dominated = class_ids.iter().any(|&d| d != c && reach[d][c]);

        if !dominated {
            let mut members: Vec<CapabilityId> = members_by_class[&c]
                .iter()
                .map(|&idx| s_list[idx].clone())
                .collect();
            members.sort_by(|a, b| a.0.cmp(&b.0));
            let member_set: HashSet<CapabilityId> = members.iter().cloned().collect();
            let entries = compute_region_entries(source, adjacency, &member_set, branch_roots);
            regions.push(StructuralConvergenceRegion { members, entries });
        }
    }

    regions.sort_by(|a, b| a.members[0].0.cmp(&b.members[0].0));
    regions
}

/// PATH-031: cycle-aware replacement candidate for
/// `first_structural_convergences`, over the RAW shared-bottleneck set.
/// Where `first_structural_convergences` collapses to `[]` when the
/// shared set contains a cycle (PATH-028-f), this reports the cycle as one
/// `StructuralConvergenceRegion` instead of erasing it. On acyclic shared
/// sets the two agree in shape (one region per first-convergence node).
/// `first_structural_convergences` itself is unchanged and remains
/// available -- this is an additional candidate, not a replacement.
pub fn cycle_aware_first_structural_convergences(
    source: &CapabilityId,
    edges: &[DependencyEdge],
) -> Vec<StructuralConvergenceRegion> {
    let adjacency = build_structural_adjacency(edges);
    let branch_roots = immediate_branch_roots(source, &adjacency);
    let shared: std::collections::HashSet<CapabilityId> = shared_structural_bottlenecks(source, edges)
        .into_iter()
        .collect();
    cycle_aware_first_convergences_over_shared(source, &adjacency, shared, &branch_roots)
}

/// PATH-031 composed with PATH-030: cycle-aware first-convergence regions
/// computed over `shared_structural_bottlenecks_from_effective_branches`
/// instead of the raw shared-bottleneck set.
///
/// PATH-032: entries are attributed to EFFECTIVE branches
/// (`effective_structural_branches`), not raw immediate ones -- a raw
/// branch root that PATH-029 would drop as redundant must not manufacture
/// entry evidence here either.
pub fn cycle_aware_first_structural_convergences_from_effective_branches(
    source: &CapabilityId,
    edges: &[DependencyEdge],
) -> Vec<StructuralConvergenceRegion> {
    let adjacency = build_structural_adjacency(edges);
    let branch_roots = effective_structural_branches(source, edges);
    let shared: std::collections::HashSet<CapabilityId> =
        shared_structural_bottlenecks_from_effective_branches(source, edges)
            .into_iter()
            .collect();
    cycle_aware_first_convergences_over_shared(source, &adjacency, shared, &branch_roots)
}

/// Candidate ECONOMIC REDUCTION of a `StructuralConvergenceRegion` down to
/// a single scalar (PATH-033).
///
/// H4 under test: economic structural importance depends primarily on the
/// number of DISTINCT branches (`entries[i].branch`, deduplicated) that
/// enter a first-convergence region -- not on `members.len()` (how big the
/// converged SCC is) and not on how many separate entries a single branch
/// contributes (`path_032_e`-style repeated entries from one branch). This
/// function computes exactly that count and nothing else.
///
/// It is deliberately the smallest possible reduction: it does not look at
/// `members` at all, and it counts each distinct `branch` once regardless
/// of how many entries that branch has. Whether `branch` here is a raw
/// immediate branch or a PATH-030 effective branch is entirely determined
/// by which region-producing function the caller used upstream -- this
/// function has no opinion and does no subsumption analysis of its own;
/// composing with `cycle_aware_first_structural_convergences_from_effective_
/// branches` is the PATH-030-recommended choice when redundant raw
/// branches must not inflate the count (see `path_033_d`).
///
/// This is a CANDIDATE under adversarial test, not an established result:
/// see the PATH-033 test group for cases attacking whether member count
/// and entry multiplicity are truly economically inert, and FIND-010 for
/// the outcome. It is NOT wired into any `aggregate_*` function or money
/// allocation -- `members` and `entries` remain the raw, inspectable
/// structural evidence; this is one candidate lens on top of them, not a
/// replacement for them.
pub fn distinct_entering_branch_count(region: &StructuralConvergenceRegion) -> usize {
    let branches: std::collections::HashSet<&CapabilityId> =
        region.entries.iter().map(|entry| &entry.branch).collect();
    branches.len()
}

/// Candidate SUBSUMED-BRANCH semantics for structural independence
/// (PATH-029).
///
/// PATH-027/028 assumed that `source`'s distinct immediate out-edges are
/// each an independent "branch". PATH-029 attacks that assumption with:
///
/// ```text
/// A -> B -> C
/// A -> C -> D
/// ```
///
/// `B` and `C` are both immediate children of `A`, so the earlier
/// primitives count two branches. But `B`'s only route to anything is
/// through `C`: remove `C` and `B` reaches nothing beyond itself. `C`'s
/// reach does not depend on `B` at all. So `B` contributes no structural
/// independence beyond what `C` already provides -- `B` is *subsumed* by
/// `C`.
///
/// ## Definition
///
/// For two distinct branch roots `r1`, `r2`, `r1` is subsumed by `r2`
/// when:
/// 1. `r2` is reachable from `r1`, AND
/// 2. deleting `r2` from the graph (treating it as a dead end) leaves
///    `r1` unable to reach anything other than itself and `r2`.
///
/// This is deliberately NOT plain reachable-set containment
/// (`reachable(r1) ⊆ reachable(r2)`). On the example above, plain
/// containment gets the direction backwards: `reachable(C) = {C, D}` is
/// the *smaller* set and is a subset of `reachable(B) = {B, C, D}`, which
/// would flag `C` -- the branch that needs nothing from `B` -- as the
/// subsumed one. The counterfactual-removal definition above correctly
/// identifies `B`, the branch whose entire value is gatekept by `C`, as
/// the redundant one. See `path_029_counterexample_naive_containment_is_backwards`.
///
/// This definition also naturally preserves genuine partial independence:
/// if `B` reaches `C`'s territory but *also* has unique reachable nodes
/// that do not depend on `C` (e.g. `A->B->X, A->C->X, B->Y`), condition 2
/// fails (`Y` survives `C`'s removal), so `B` is not subsumed.
///
/// ## Cyclic subsumption among branch roots
///
/// Two branch roots can subsume each other (a 2-cycle: `A->B, A->C, B->C,
/// C->B`), and PATH-029 found that three or more branch roots can form a
/// longer cycle of *one-directional* subsumption (`B` subsumed by `C`,
/// `C` subsumed by `E`, `E` subsumed by `B`) that is not pairwise mutual
/// at any single edge. Dropping every branch that is subsumed by *some*
/// other branch, without accounting for such cycles, collapses the whole
/// cycle to an empty result -- three genuine branches vanishing entirely
/// (see `path_029_d2_three_way_subsumption_cycle_would_collapse_to_empty_without_scc_handling`).
/// That is treated as a real defect, not an accepted ambiguity: the fix is
/// a transitive closure over the (small, bounded-by-source-out-degree)
/// subsumption relation among branch roots, grouping any set of mutually
/// reaching branch roots into one equivalence class and keeping a single
/// deterministic representative (the lexicographically smallest
/// `CapabilityId`) per class. This is a narrowly scoped SCC condensation
/// of the subsumption relation *among branch roots only* -- not a
/// general-purpose graph SCC library over the whole capability graph.
pub fn effective_structural_branches(
    source: &CapabilityId,
    edges: &[DependencyEdge],
) -> Vec<CapabilityId> {
    use std::collections::{HashMap, HashSet, VecDeque};

    let mut adjacency: HashMap<CapabilityId, Vec<CapabilityId>> = HashMap::new();
    let mut seen_edges: HashSet<(CapabilityId, CapabilityId)> = HashSet::new();
    for edge in edges {
        let key = (edge.source.clone(), edge.target.clone());
        if seen_edges.insert(key) {
            adjacency
                .entry(edge.source.clone())
                .or_default()
                .push(edge.target.clone());
        }
    }

    let mut branch_roots: Vec<CapabilityId> = Vec::new();
    let mut seen_roots: HashSet<CapabilityId> = HashSet::new();
    if let Some(children) = adjacency.get(source) {
        for child in children {
            if child != source && seen_roots.insert(child.clone()) {
                branch_roots.push(child.clone());
            }
        }
    }

    if branch_roots.len() < 2 {
        branch_roots.sort_by(|a, b| a.0.cmp(&b.0));
        return branch_roots;
    }

    // subsumed[i][j]: is branch_roots[i] subsumed by branch_roots[j]?
    // BFS from root_i with both `source` and `root_j` treated as dead
    // ends (source, to keep branches isolated from each other as in
    // shared_structural_bottlenecks; root_j, to simulate its removal).
    let n = branch_roots.len();
    let mut subsumed = vec![vec![false; n]; n];

    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let root_i = &branch_roots[i];
            let root_j = &branch_roots[j];

            let mut visited: HashSet<CapabilityId> = HashSet::new();
            let mut queue: VecDeque<CapabilityId> = VecDeque::new();
            visited.insert(root_i.clone());
            queue.push_back(root_i.clone());

            while let Some(node) = queue.pop_front() {
                if node == *source || node == *root_j {
                    continue;
                }
                if let Some(children) = adjacency.get(&node) {
                    for child in children {
                        if visited.insert(child.clone()) {
                            queue.push_back(child.clone());
                        }
                    }
                }
            }

            let reaches_root_j = visited.contains(root_j);
            let remainder = visited
                .iter()
                .filter(|n| *n != root_i && *n != root_j)
                .count();

            subsumed[i][j] = reaches_root_j && remainder == 0;
        }
    }

    // Transitive closure of the subsumption relation, so cycles of
    // one-directional subsumption among three or more branch roots are
    // detected as a single equivalence class rather than each branch
    // dropping the next until nothing is left.
    let mut reach = subsumed.clone();
    for k in 0..n {
        for i in 0..n {
            if !reach[i][k] {
                continue;
            }
            for j in 0..n {
                if reach[k][j] {
                    reach[i][j] = true;
                }
            }
        }
    }

    let mut keep = vec![true; n];
    for i in 0..n {
        let mut group_min = branch_roots[i].0.clone();
        for j in 0..n {
            if i != j && reach[i][j] && reach[j][i] && branch_roots[j].0 < group_min {
                group_min = branch_roots[j].0.clone();
            }
        }

        if branch_roots[i].0 != group_min {
            // Not the canonical representative of its equivalence class.
            keep[i] = false;
            continue;
        }

        // Canonical within its own class; drop if a genuinely different
        // (non-mutual) class strictly dominates it.
        for j in 0..n {
            if i == j {
                continue;
            }
            let mutual = reach[i][j] && reach[j][i];
            if !mutual && reach[i][j] {
                keep[i] = false;
                break;
            }
        }
    }

    let mut effective: Vec<CapabilityId> = (0..n)
        .filter(|&i| keep[i])
        .map(|i| branch_roots[i].clone())
        .collect();

    effective.sort_by(|a, b| a.0.cmp(&b.0));
    effective
}

/// Explicit counterfactual removal (PATH-036): `edges` with every edge
/// incident to `removed` (as either source or target) dropped.
///
/// This is deliberately "delete the node and all its incident edges" --
/// there is no separate "delete only outgoing" or "delete only incoming"
/// variant, because a `CapabilityId` with dangling half-edges is not a
/// meaningful graph state to reason about here. This is the ONE removal
/// semantics PATH-036 uses everywhere; no second, inconsistent notion of
/// "remove X" is introduced elsewhere in this module.
///
/// Deterministic, cycle-safe (it is a pure filter, not a traversal),
/// label-independent, and free of any economic/substitutability/capacity
/// inference: it only ever compares `CapabilityId` equality.
pub fn remove_capability(edges: &[DependencyEdge], removed: &CapabilityId) -> Vec<DependencyEdge> {
    edges
        .iter()
        .filter(|edge| edge.source != *removed && edge.target != *removed)
        .cloned()
        .collect()
}

/// Plain topological reachability (PATH-036): is `target` reachable from
/// `source` by following `edges` forward? Cycle-safe via a visited set;
/// carries no operational meaning beyond "a directed path exists."
pub fn is_reachable(source: &CapabilityId, target: &CapabilityId, edges: &[DependencyEdge]) -> bool {
    use std::collections::{HashMap, HashSet, VecDeque};

    if source == target {
        return true;
    }

    let mut adjacency: HashMap<CapabilityId, Vec<CapabilityId>> = HashMap::new();
    for edge in edges {
        adjacency
            .entry(edge.source.clone())
            .or_default()
            .push(edge.target.clone());
    }

    let mut visited: HashSet<CapabilityId> = HashSet::new();
    let mut queue: VecDeque<CapabilityId> = VecDeque::new();
    visited.insert(source.clone());
    queue.push_back(source.clone());

    while let Some(node) = queue.pop_front() {
        if let Some(children) = adjacency.get(&node) {
            for child in children {
                if child == target {
                    return true;
                }
                if visited.insert(child.clone()) {
                    queue.push_back(child.clone());
                }
            }
        }
    }

    false
}

/// PATH-036 CORRECTION (target-scope leak): `effective_structural_
/// branches(source, edges)` is NOT scoped to any particular `target` --
/// it reports every one of `source`'s distinct (subsumption-deduped)
/// branches, regardless of where each one leads. An earlier version of
/// `StructuralCounterfactual` used that source-global count directly
/// alongside a `source -> target`-specific reachability question. That
/// let a branch entirely unrelated to `target` inflate or deflate the
/// apparent before/after delta reported *for that target*: e.g.
/// `SOURCE->X->ANALYTICS, SOURCE->Y->PAYMENT`, querying
/// `target = PAYMENT`, `removed = X` -- the source-global count drops
/// 2 -> 1 (X's branch to ANALYTICS is gone), even though X never lay on
/// any `SOURCE -> PAYMENT` path and PAYMENT's own redundancy is
/// completely unaffected. See `path_036_m_*` for the confirmed attack
/// and FIND-013 for the full correction.
///
/// This function fixes that by filtering `effective_structural_branches`
/// down to only the branches that can actually reach `target` at all
/// ([`is_reachable`]) -- relevance comes from graph reachability, never
/// from labels. It is NOT a complete redundancy-quality measure even
/// after scoping: two branches can both reach `target` and still
/// reconverge at a shared downstream bottleneck before getting there
/// (PATH-036-D/L already demonstrate this, via
/// `first_structural_convergences`), and this function has no opinion
/// about that -- it only counts distinct branch roots that reach
/// `target`, nothing about what they share once there.
pub fn target_scoped_effective_branches(
    source: &CapabilityId,
    target: &CapabilityId,
    edges: &[DependencyEdge],
) -> Vec<CapabilityId> {
    effective_structural_branches(source, edges)
        .into_iter()
        .filter(|branch| is_reachable(branch, target, edges))
        .collect()
}

/// Minimal deterministic STRUCTURAL counterfactual (PATH-036): what
/// changes, in purely topological/structural terms, about `source`'s
/// relationship to `target` when `removed` is deleted from the graph
/// (via [`remove_capability`]).
///
/// All four fields describe `source`'s relationship to `target`
/// specifically -- none is a source-global quantity:
/// - `before_reachable`/`after_reachable`: whether `target` is reachable
///   from `source` at all ([`is_reachable`]).
/// - `before_effective_branches_to_target`/
///   `after_effective_branches_to_target`: how many of `source`'s
///   distinct effective branches can reach `target`
///   ([`target_scoped_effective_branches`] -- PATH-029/030's
///   subsumption-aware branch count, filtered by reachability to
///   `target`). See the PATH-036 correction note on
///   `target_scoped_effective_branches` for why this filtering is
///   required and what it still does not establish (it is not a
///   complete redundancy-quality measure -- PATH-036-D/L).
///
/// It does NOT report, compute, or infer: capacity, latency, redundancy
/// reserve, substitution delay, partial/degraded functionality, business
/// value, or any economic score. `bool`/`usize` fields only -- no floats,
/// no weights. See PATH-036 research notes (`tests/pathological_cases.rs`,
/// `path_036_*`) and FIND-013 for what this type does and does not
/// establish. It is NOT wired into `CriticalityObservables`,
/// `measure_criticality`, `Contribution`, or any `aggregate_*` function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuralCounterfactual {
    pub before_reachable: bool,
    pub after_reachable: bool,
    pub before_effective_branches_to_target: usize,
    pub after_effective_branches_to_target: usize,
}

pub fn structural_counterfactual(
    source: &CapabilityId,
    target: &CapabilityId,
    edges: &[DependencyEdge],
    removed: &CapabilityId,
) -> StructuralCounterfactual {
    let after_edges = remove_capability(edges, removed);

    StructuralCounterfactual {
        before_reachable: is_reachable(source, target, edges),
        after_reachable: is_reachable(source, target, &after_edges),
        before_effective_branches_to_target: target_scoped_effective_branches(
            source, target, edges,
        )
        .len(),
        after_effective_branches_to_target: target_scoped_effective_branches(
            source,
            target,
            &after_edges,
        )
        .len(),
    }
}

/// All `CapabilityId`s reachable from `source` by following `edges`
/// forward, EXCLUDING `source` itself (PATH-037). Cycle-safe via a
/// visited-set BFS (the same traversal shape as [`is_reachable`]).
/// Sorted lexically by `CapabilityId` value for deterministic output --
/// that ordering exists purely for stable enumeration/serialization and
/// carries no structural or economic meaning.
///
/// This is topology-only: a `CapabilityId` that never appears as the
/// source or target of any edge in `edges` cannot appear here, even if
/// some larger "declared capability universe" outside this graph
/// representation considers it to exist. This edge-only representation
/// cannot express a capability with no relationships at all -- that
/// limitation is inherited by every function built on top of this one,
/// including [`structural_propagation_surface`].
pub fn reachable_capabilities(source: &CapabilityId, edges: &[DependencyEdge]) -> Vec<CapabilityId> {
    use std::collections::{HashMap, HashSet, VecDeque};

    let mut adjacency: HashMap<CapabilityId, Vec<CapabilityId>> = HashMap::new();
    for edge in edges {
        adjacency
            .entry(edge.source.clone())
            .or_default()
            .push(edge.target.clone());
    }

    let mut visited: HashSet<CapabilityId> = HashSet::new();
    let mut queue: VecDeque<CapabilityId> = VecDeque::new();
    visited.insert(source.clone());
    queue.push_back(source.clone());

    while let Some(node) = queue.pop_front() {
        if let Some(children) = adjacency.get(&node) {
            for child in children {
                if visited.insert(child.clone()) {
                    queue.push_back(child.clone());
                }
            }
        }
    }

    visited.remove(source);
    let mut result: Vec<CapabilityId> = visited.into_iter().collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

/// One target's structural transition inside a [`structural_propagation_
/// surface`] result (PATH-037). Field meanings are identical to
/// [`StructuralCounterfactual`]'s, scoped to `target`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralPropagationEntry {
    pub target: CapabilityId,
    pub before_reachable: bool,
    pub after_reachable: bool,
    pub before_effective_branches_to_target: usize,
    pub after_effective_branches_to_target: usize,
}

/// Multi-target structural propagation surface of removing `removed`
/// from `edges` (PATH-037): the [`StructuralCounterfactual`] of every
/// target in `source`'s PRE-REMOVAL reachable set, for targets whose
/// structural state actually changed.
///
/// ## Target universe
///
/// The candidate target universe is `reachable_capabilities(source,
/// edges)` (every `CapabilityId` reachable from `source` before
/// removal), with `source` and `removed` excluded:
///
/// - `source` is excluded because `is_reachable` treats a node as
///   trivially reachable from itself, so an entry for `source` would
///   always report `before_reachable == after_reachable == true` with no
///   informative content.
/// - `removed` is excluded because the query is about DOWNSTREAM
///   CONSEQUENCES of removing it, not the tautological fact that the
///   removed capability itself is now gone. If `removed` was reachable
///   from `source` before removal, it deterministically becomes
///   unreachable after (nothing can point into a node with all its
///   incident edges stripped, except `source` itself in the degenerate
///   `removed == source` case) -- that transition is guaranteed by
///   construction for ANY removed capability and reports nothing about
///   propagation to OTHER capabilities.
/// - A target that was NOT reachable before removal is never included,
///   and could not usefully be: `remove_capability` only ever deletes
///   edges, and deleting edges from a graph cannot create a directed
///   path that did not already exist (every path usable in the
///   post-removal edge set is, by construction, also usable in the
///   pre-removal one, since post-removal edges are a subset of
///   pre-removal edges). Reachability can therefore only be lost or
///   preserved by removal, never gained -- see `path_037_inspection_*`
///   for an executable check of this on a graph containing a cycle.
///
/// A target is included in the returned `Vec` only if its
/// `StructuralCounterfactual` (built with [`structural_counterfactual`])
/// differs from the trivial "before" baseline in `after_reachable` or
/// `after_effective_branches_to_target` -- i.e. only targets whose
/// structural state actually changed. Because every candidate target is
/// drawn from `source`'s pre-removal reachable set, `before_reachable`
/// is always `true` and `before_effective_branches_to_target` is always
/// at least 1 for every entry this function can ever produce; the
/// interesting information is entirely in the `after_*` fields.
///
/// Entries are sorted lexically by `target` for deterministic output;
/// that ordering is for stable enumeration only and carries no
/// structural or economic meaning.
///
/// This is NOT a complete summary of removal consequences: it does not
/// incorporate `shared_structural_bottlenecks` or
/// `first_structural_convergences`, so it inherits the same blind spot
/// `StructuralCounterfactual` has for a shared bottleneck the removed
/// capability does not itself sit on (PATH-036-D/L; see `path_037_f`).
/// It also does NOT reduce to a scalar, and PATH-037 found no scalar
/// reduction of it (count of changed targets, count of targets losing
/// reachability, count of targets losing a branch while remaining
/// reachable) that is representation-invariant under chain
/// decomposition or relay insertion -- see `path_037_q*` and FIND-014.
/// No floats, no weights, no severity, no economic meaning anywhere in
/// this type or function.
pub fn structural_propagation_surface(
    source: &CapabilityId,
    edges: &[DependencyEdge],
    removed: &CapabilityId,
) -> Vec<StructuralPropagationEntry> {
    let mut targets = reachable_capabilities(source, edges);
    targets.retain(|target| target != removed);

    let mut surface: Vec<StructuralPropagationEntry> = targets
        .into_iter()
        .filter_map(|target| {
            let result = structural_counterfactual(source, &target, edges, removed);

            let changed = result.before_reachable != result.after_reachable
                || result.before_effective_branches_to_target
                    != result.after_effective_branches_to_target;

            if !changed {
                return None;
            }

            Some(StructuralPropagationEntry {
                target,
                before_reachable: result.before_reachable,
                after_reachable: result.after_reachable,
                before_effective_branches_to_target: result.before_effective_branches_to_target,
                after_effective_branches_to_target: result.after_effective_branches_to_target,
            })
        })
        .collect();

    surface.sort_by(|a, b| a.target.0.cmp(&b.target.0));
    surface
}

/// PATH-038 provenance record: one node collapsed by
/// [`canonicalize_pure_relays`], with its predecessor/successor AT THE
/// MOMENT it was collapsed. Recording the predecessor/successor (not
/// just the collapsed node's id) is what makes a multi-hop chain fully
/// reconstructable: for a chain `A -> R1 -> R2 -> TARGET` collapsed in
/// order R1 then R2, the two entries `{R1, A, R2}` and `{R2, A, TARGET}`
/// together recover the full original chain, even though R2 no longer
/// existed as an edge target by the time it was itself collapsed. No
/// entry is ever silently dropped -- every collapsed node has exactly
/// one entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayCollapseProvenance {
    pub collapsed: CapabilityId,
    pub predecessor: CapabilityId,
    pub successor: CapabilityId,
}

/// PATH-038 experimental canonicalization: collapse PURE RELAY nodes out
/// of `edges`, relative to `source` and an explicit `protected` set,
/// returning the canonical edge list plus complete collapse provenance.
///
/// ## The contract (topology-only, exactly four conditions)
///
/// A node is eligible for collapse only if ALL of the following hold,
/// determined purely from `edges` (never from labels):
/// 1. it is not `source`;
/// 2. it is not in the caller-supplied `protected` set;
/// 3. it has EXACTLY ONE distinct predecessor and EXACTLY ONE distinct
///    successor (duplicate edges do not count twice; a node with zero or
///    two-or-more of either is never eligible -- this is what makes a
///    branch point, per PATH-038-C, and a convergence point, per
///    PATH-038-D, structurally incapable of ever being collapsed: a
///    branch point has >= 2 successors, a convergence point has >= 2
///    predecessors);
/// 4. it does not lie on a cycle: its sole successor must not be able to
///    reach it again ([`is_reachable`]). This is what makes
///    `A -> B -> C -> A` (PATH-038-G) collapse to nothing at all, even
///    though every member has exactly one predecessor and successor --
///    collapsing any one of them would silently destroy the cycle's
///    real topology (turning a 3-cycle into a 2-cycle, then a
///    self-loop), which is not a case this contract accepts.
///
/// Collapsing an eligible node `R` with predecessor `P` and successor
/// `S` removes `P -> R` and `R -> S` and adds `P -> S` (deduplicated
/// against any edge already present). This is repeated to a fixed point
/// (recomputing eligibility fresh after every single collapse, one node
/// at a time, in lexical order among that pass's candidates, for
/// determinism), so a multi-hop relay chain fully collapses in one call
/// -- see PATH-038-L. Calling this function again on its own output is a
/// no-op on the returned edges (idempotent), because after reaching a
/// fixed point no node can satisfy all four conditions.
///
/// ## What this does NOT establish
///
/// This is a TOPOLOGY-ONLY mechanism. It cannot, and does not attempt
/// to, determine whether a degree-(1,1) non-cyclic node is a genuine
/// implementation pass-through or an economically meaningful capability
/// that simply happens to sit between exactly one predecessor and one
/// successor -- **those two situations are topologically identical**
/// (PATH-038-N/P). The `protected` parameter is the ONLY thing standing
/// between a real capability and a false collapse: any node the caller
/// does not think to protect, and that happens to have this shape, is
/// collapsed unconditionally. This function does not claim, and PATH-038
/// did not establish, that the canonical graph it produces is
/// economically correct, unique, or that topology reveals semantic
/// capability identity. See FIND-015 for the full verdict.
///
/// No floats, no weights, no severity, no economic meaning anywhere in
/// this function.
pub fn canonicalize_pure_relays(
    source: &CapabilityId,
    protected: &[CapabilityId],
    edges: &[DependencyEdge],
) -> (Vec<DependencyEdge>, Vec<RelayCollapseProvenance>) {
    use std::collections::HashSet;

    let protected_set: HashSet<CapabilityId> = protected.iter().cloned().collect();

    collapse_relays_where(source, edges, |node| !protected_set.contains(node))
}

/// Shared fixed-point collapse core for [`canonicalize_pure_relays`]
/// (PATH-038) and [`canonicalize_declared_representation_nodes`]
/// (PATH-039). `policy_allows(node)` decides whether `node` is even a
/// CANDIDATE for collapse, given whatever caller-specific policy applies
/// (a plain protected-set membership test for PATH-038; that same test
/// AND an explicit semantic authorization for PATH-039) -- `source`
/// itself is excluded unconditionally regardless of policy, matching
/// PATH-038-F. Every other guard (exactly one predecessor, exactly one
/// successor, not on a cycle) is universal topology and lives here,
/// identically, for both callers -- this refactor changes neither
/// function's observable behavior, only where the shared loop lives.
fn collapse_relays_where(
    source: &CapabilityId,
    edges: &[DependencyEdge],
    policy_allows: impl Fn(&CapabilityId) -> bool,
) -> (Vec<DependencyEdge>, Vec<RelayCollapseProvenance>) {
    use std::collections::{HashMap, HashSet};

    let mut current: HashSet<(CapabilityId, CapabilityId)> = edges
        .iter()
        .map(|edge| (edge.source.clone(), edge.target.clone()))
        .collect();

    let mut provenance: Vec<RelayCollapseProvenance> = Vec::new();

    loop {
        let mut predecessors: HashMap<CapabilityId, HashSet<CapabilityId>> = HashMap::new();
        let mut successors: HashMap<CapabilityId, HashSet<CapabilityId>> = HashMap::new();
        let mut nodes: HashSet<CapabilityId> = HashSet::new();

        for (edge_source, edge_target) in &current {
            nodes.insert(edge_source.clone());
            nodes.insert(edge_target.clone());
            successors
                .entry(edge_source.clone())
                .or_default()
                .insert(edge_target.clone());
            predecessors
                .entry(edge_target.clone())
                .or_default()
                .insert(edge_source.clone());
        }

        let current_edges_vec: Vec<DependencyEdge> = current
            .iter()
            .map(|(edge_source, edge_target)| DependencyEdge {
                source: edge_source.clone(),
                target: edge_target.clone(),
            })
            .collect();

        let mut candidates: Vec<CapabilityId> = Vec::new();

        for node in &nodes {
            if node == source || !policy_allows(node) {
                continue;
            }

            let (preds, succs) = match (predecessors.get(node), successors.get(node)) {
                (Some(p), Some(s)) => (p, s),
                _ => continue,
            };

            if preds.len() != 1 || succs.len() != 1 {
                continue;
            }

            let predecessor = preds.iter().next().unwrap();
            let successor = succs.iter().next().unwrap();

            if predecessor == node || successor == node {
                continue;
            }

            if is_reachable(successor, node, &current_edges_vec) {
                continue;
            }

            candidates.push(node.clone());
        }

        if candidates.is_empty() {
            break;
        }

        candidates.sort_by(|a, b| a.0.cmp(&b.0));
        let chosen = candidates.into_iter().next().unwrap();

        let predecessor = predecessors[&chosen].iter().next().unwrap().clone();
        let successor = successors[&chosen].iter().next().unwrap().clone();

        current.remove(&(predecessor.clone(), chosen.clone()));
        current.remove(&(chosen.clone(), successor.clone()));
        current.insert((predecessor.clone(), successor.clone()));

        provenance.push(RelayCollapseProvenance {
            collapsed: chosen,
            predecessor,
            successor,
        });
    }

    let mut canonical_edges: Vec<DependencyEdge> = current
        .into_iter()
        .map(|(edge_source, edge_target)| DependencyEdge {
            source: edge_source,
            target: edge_target,
        })
        .collect();
    canonical_edges.sort_by(|a, b| a.source.0.cmp(&b.source.0).then(a.target.0.cmp(&b.target.0)));

    (canonical_edges, provenance)
}

/// PATH-039: an explicit, topology-independent claim about one
/// `CapabilityId`'s representational role.
///
/// Deliberately two variants only, NOT three: `Unclassified` is not a
/// value anyone can meaningfully assert ("I explicitly claim this node
/// is unclassified" says nothing), so it is represented by the ABSENCE
/// of any `RepresentationClaim` for a given capability in the evidence
/// set passed to [`evaluate_representation_claims`], not by a third enum
/// variant. This keeps "no one has said anything about this node" and
/// "someone explicitly said X" structurally distinguishable, which a
/// bare `HashSet<CapabilityId>` membership test cannot do -- see
/// `path_039_candidate_a_*` for why that bare-boolean strawman
/// (Candidate A, PATH-039 STEP 6) was rejected in favor of this shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentationRole {
    SemanticCapability,
    RepresentationOnly,
}

/// Identity of one representation claim's supporting evidence (PATH-039
/// Candidate B, STEP 6). This repository has no organizational-authority
/// or issuer-identity type to reuse (inspected: `DependencyId`,
/// `DependencyPathId`, `FailureDomainId` are all bare string identifiers
/// with no issuer/authority semantics either), so none is invented here
/// -- this is exactly as much identity as every other `XxxId(String)`
/// type in this module carries, no more. An empty string is treated as
/// MISSING/INVALID evidence by [`evaluate_representation_claims`]
/// (PATH-039-Q), not as a valid claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentationClaimId(pub String);

/// One explicit representation-role claim (PATH-039). `role` must be
/// asserted by SOME evidence identity (`claim_id`) -- this is Candidate
/// B from STEP 6 (declaration + evidence id), not Candidate A (a bare
/// boolean). Nothing about this type verifies the claim is TRUE; see
/// `path_039_d`/`path_039_l` for why topology-only canonicalization
/// cannot detect a false claim, and FIND-016 for the full verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentationClaim {
    pub capability: CapabilityId,
    pub role: RepresentationRole,
    pub claim_id: RepresentationClaimId,
}

/// Deterministic outcome of evaluating every claim about one capability
/// (PATH-039 STEP 7). No arbitrary tie-breaking, no "last writer wins,"
/// no timestamp-based resolution: `Conflicted` is returned, visibly,
/// whenever both roles are asserted for the same capability by any
/// claims, rather than silently picking one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentationDecision {
    /// At least one valid `RepresentationOnly` claim exists, no
    /// `SemanticCapability` claim exists, and no claim for this
    /// capability has invalid evidence.
    Authorized,
    /// Covers three distinct underlying situations, deliberately
    /// collapsed into one outcome because all three have the identical
    /// practical effect (never authorize collapse): no claim exists at
    /// all (Unclassified, conservative default); an explicit
    /// `SemanticCapability` claim exists; or a `RepresentationOnly`
    /// claim exists but its evidence is missing/invalid (empty
    /// `claim_id`).
    Refused,
    /// Both `SemanticCapability` and `RepresentationOnly` are asserted
    /// for the same capability (by any claims, regardless of how many).
    /// Never authorizes collapse; always reported back to the caller via
    /// [`representation_claim_conflicts`] rather than resolved silently.
    Conflicted,
}

/// Evaluate every claim in `claims` about `capability` and return the
/// single deterministic [`RepresentationDecision`]. See that type's
/// variants for the exact rules. Duplicate claims (same or different
/// `claim_id`, same role) do not change the outcome -- authority does
/// not accumulate with repetition (PATH-039-J/M).
pub fn evaluate_representation_claims(
    capability: &CapabilityId,
    claims: &[RepresentationClaim],
) -> RepresentationDecision {
    let relevant: Vec<&RepresentationClaim> = claims
        .iter()
        .filter(|claim| &claim.capability == capability)
        .collect();

    if relevant.is_empty() {
        return RepresentationDecision::Refused;
    }

    let has_semantic = relevant
        .iter()
        .any(|claim| claim.role == RepresentationRole::SemanticCapability);
    let has_representation_only = relevant
        .iter()
        .any(|claim| claim.role == RepresentationRole::RepresentationOnly);

    if has_semantic && has_representation_only {
        return RepresentationDecision::Conflicted;
    }

    if has_semantic {
        return RepresentationDecision::Refused;
    }

    let all_representation_only_claims_valid = relevant
        .iter()
        .filter(|claim| claim.role == RepresentationRole::RepresentationOnly)
        .all(|claim| !claim.claim_id.0.is_empty());

    if !all_representation_only_claims_valid {
        return RepresentationDecision::Refused;
    }

    RepresentationDecision::Authorized
}

/// Every distinct capability among `claims` whose
/// [`evaluate_representation_claims`] result is `Conflicted`, sorted for
/// determinism. Exists so a caller can always inspect conflicts
/// independent of whether any conflicted node ever became a topology-
/// eligible collapse candidate.
pub fn representation_claim_conflicts(claims: &[RepresentationClaim]) -> Vec<CapabilityId> {
    use std::collections::HashSet;

    let distinct_capabilities: HashSet<CapabilityId> =
        claims.iter().map(|claim| claim.capability.clone()).collect();

    let mut conflicted: Vec<CapabilityId> = distinct_capabilities
        .into_iter()
        .filter(|capability| {
            evaluate_representation_claims(capability, claims) == RepresentationDecision::Conflicted
        })
        .collect();

    conflicted.sort_by(|a, b| a.0.cmp(&b.0));
    conflicted
}

/// PATH-039: semantics-gated variant of [`canonicalize_pure_relays`]. A
/// node is eligible for collapse only if it clears EVERY guard
/// `canonicalize_pure_relays` requires (not `source`, not in
/// `protected`, exactly one predecessor, exactly one successor, not on a
/// cycle) AND `evaluate_representation_claims` returns `Authorized` for
/// it. Semantic authorization alone is never sufficient on its own --
/// PATH-039-E/F confirm a branch or convergence point marked
/// `RepresentationOnly` still never collapses, because it can never
/// satisfy the one-predecessor/one-successor shape regardless of policy.
///
/// Returns the canonical edges, full collapse provenance (identical
/// shape to PATH-038's), and every conflicted capability found in
/// `claims` (via [`representation_claim_conflicts`]) -- conflicts are
/// always reported, never used to silently block or permit unrelated
/// collapses elsewhere in the graph.
///
/// This function does NOT verify that any `RepresentationOnly` claim is
/// TRUE. PATH-039-D/L demonstrate that a structurally valid, correctly-
/// evidenced claim about a capability that is actually semantically
/// meaningful is indistinguishable, from this function's perspective,
/// from a claim about a genuine relay -- verifying truth requires
/// evidence this repository does not have and this experiment does not
/// invent. PATH-039-O further demonstrates that even fully TRUTHFUL,
/// individually-correct claims can still reproduce PATH-038-K/P's
/// cascading branch-multiplicity erasure at an unrelated, unclaimed
/// convergence node -- explicit semantics fix the "topology alone can't
/// tell relay from capability" problem (PATH-038-N), not the separate
/// "independent bypass edges can deduplicate onto a shared ancestor"
/// problem. See FIND-016 for the full verdict.
pub fn canonicalize_declared_representation_nodes(
    source: &CapabilityId,
    protected: &[CapabilityId],
    claims: &[RepresentationClaim],
    edges: &[DependencyEdge],
) -> (Vec<DependencyEdge>, Vec<RelayCollapseProvenance>, Vec<CapabilityId>) {
    use std::collections::HashSet;

    let protected_set: HashSet<CapabilityId> = protected.iter().cloned().collect();
    let conflicts = representation_claim_conflicts(claims);

    let (canonical_edges, provenance) = collapse_relays_where(source, edges, |node| {
        !protected_set.contains(node)
            && evaluate_representation_claims(node, claims) == RepresentationDecision::Authorized
    });

    (canonical_edges, provenance, conflicts)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DependencyPathId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FailureDomainId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DependencyRelationshipType {
    Runtime,
    DataFlow,
    ControlFlow,
    Operational,
    BusinessPath,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DependencyRecord {
    pub dependency_id: DependencyId,
    pub dependency_path_id: DependencyPathId,
    pub failure_domain_id: FailureDomainId,
    pub source_capability: CapabilityId,
    pub target_capability: CapabilityId,
    pub relationship_type: DependencyRelationshipType,
    pub qualified: QualifiedDependency,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DependencyStructuralKey {
    pub source_capability: CapabilityId,
    pub target_capability: CapabilityId,
    pub relationship_type: DependencyRelationshipType,
}

impl DependencyRecord {
    pub fn structural_key(&self) -> DependencyStructuralKey {
        DependencyStructuralKey {
            source_capability: self.source_capability.clone(),
            target_capability: self.target_capability.clone(),
            relationship_type: self.relationship_type,
        }
    }
}

pub fn deduplicate_structural_dependencies(records: &[DependencyRecord]) -> Vec<DependencyRecord> {
    use std::collections::HashMap;

    let mut by_structure: HashMap<DependencyStructuralKey, DependencyRecord> = HashMap::new();

    for record in records {
        let key = record.structural_key();

        match by_structure.get(&key) {
            None => {
                by_structure.insert(key, record.clone());
            }
            Some(existing) if record.qualified.score > existing.qualified.score => {
                by_structure.insert(key, record.clone());
            }
            Some(_) => {}
        }
    }

    let mut unique: Vec<_> = by_structure.into_values().collect();

    unique.sort_by(|a, b| {
        a.source_capability
            .0
            .cmp(&b.source_capability.0)
            .then(a.target_capability.0.cmp(&b.target_capability.0))
            .then(format!("{:?}", a.relationship_type).cmp(&format!("{:?}", b.relationship_type)))
    });

    unique
}

pub fn aggregate_structural_dependencies_with_model(
    records: &[DependencyRecord],
    model: DependencyAggregationModel,
) -> f64 {
    let unique = deduplicate_structural_dependencies(records);

    let qualified: Vec<QualifiedDependency> =
        unique.iter().map(|record| record.qualified).collect();

    aggregate_dependency_weight_with_model(&qualified, model)
}

pub fn deduplicate_dependency_paths(records: &[DependencyRecord]) -> Vec<DependencyRecord> {
    use std::collections::HashMap;

    let mut by_path: HashMap<DependencyPathId, DependencyRecord> = HashMap::new();

    for record in records {
        match by_path.get(&record.dependency_path_id) {
            None => {
                by_path.insert(record.dependency_path_id.clone(), record.clone());
            }
            Some(existing) if record.qualified.score > existing.qualified.score => {
                by_path.insert(record.dependency_path_id.clone(), record.clone());
            }
            Some(_) => {}
        }
    }

    let mut unique: Vec<_> = by_path.into_values().collect();

    unique.sort_by(|a, b| a.dependency_path_id.0.cmp(&b.dependency_path_id.0));

    unique
}

pub fn aggregate_dependency_paths_with_model(
    records: &[DependencyRecord],
    model: DependencyAggregationModel,
) -> f64 {
    let unique_paths = deduplicate_dependency_paths(records);

    let qualified: Vec<QualifiedDependency> =
        unique_paths.iter().map(|record| record.qualified).collect();

    aggregate_dependency_weight_with_model(&qualified, model)
}

pub fn deduplicate_failure_domains(records: &[DependencyRecord]) -> Vec<DependencyRecord> {
    use std::collections::HashMap;

    let mut by_domain: HashMap<FailureDomainId, DependencyRecord> = HashMap::new();

    for record in records {
        match by_domain.get(&record.failure_domain_id) {
            None => {
                by_domain.insert(record.failure_domain_id.clone(), record.clone());
            }
            Some(existing) if record.qualified.score > existing.qualified.score => {
                by_domain.insert(record.failure_domain_id.clone(), record.clone());
            }
            Some(_) => {}
        }
    }

    let mut unique: Vec<_> = by_domain.into_values().collect();
    unique.sort_by(|a, b| a.failure_domain_id.0.cmp(&b.failure_domain_id.0));
    unique
}

pub fn aggregate_failure_domains_with_model(
    records: &[DependencyRecord],
    model: DependencyAggregationModel,
) -> f64 {
    let unique_domains = deduplicate_failure_domains(records);

    let qualified: Vec<QualifiedDependency> = unique_domains
        .iter()
        .map(|record| record.qualified)
        .collect();

    aggregate_dependency_weight_with_model(&qualified, model)
}

pub fn deduplicate_dependency_records(records: &[DependencyRecord]) -> Vec<DependencyRecord> {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    let mut unique = Vec::new();

    for record in records {
        if seen.insert(record.dependency_id.clone()) {
            unique.push(record.clone());
        }
    }

    unique
}

pub fn aggregate_unique_dependencies_with_model(
    records: &[DependencyRecord],
    model: DependencyAggregationModel,
) -> f64 {
    let unique = deduplicate_dependency_records(records);

    let qualified: Vec<QualifiedDependency> =
        unique.iter().map(|record| record.qualified).collect();

    aggregate_dependency_weight_with_model(&qualified, model)
}

#[cfg(test)]
mod dependency_graph_tests {
    use super::*;

    fn weights() -> DependencyQualificationWeights {
        DependencyQualificationWeights {
            blocking_relevance: 0.30,
            failure_propagation: 0.30,
            operational_reliance: 0.25,
            business_path_relevance: 0.15,
        }
    }

    fn qualified(score: f64) -> QualifiedDependency {
        qualify_dependency(
            DependencyEvidence {
                blocking_relevance: score,
                failure_propagation: score,
                operational_reliance: score,
                business_path_relevance: score,
            },
            weights(),
        )
        .unwrap()
    }

    fn record(id: &str, score: f64) -> DependencyRecord {
        DependencyRecord {
            dependency_id: DependencyId(id.to_string()),
            dependency_path_id: DependencyPathId("default-path".to_string()),
            failure_domain_id: FailureDomainId("default-failure-domain".to_string()),
            source_capability: CapabilityId("capability-a".to_string()),
            target_capability: CapabilityId("capability-b".to_string()),
            relationship_type: DependencyRelationshipType::Runtime,
            qualified: qualified(score),
        }
    }

    #[test]
    fn exact_duplicate_dependency_ids_are_deduplicated() {
        let dependency = record("dep-001", 0.60);

        let records = vec![dependency.clone(), dependency.clone(), dependency.clone()];

        let unique = deduplicate_dependency_records(&records);

        assert_eq!(unique.len(), 1);
        assert_eq!(unique[0].dependency_id.0, "dep-001");
    }

    #[test]
    fn distinct_dependency_ids_are_preserved() {
        let records = vec![
            record("dep-001", 0.60),
            record("dep-002", 0.60),
            record("dep-003", 0.60),
        ];

        let unique = deduplicate_dependency_records(&records);

        assert_eq!(unique.len(), 3);
    }

    #[test]
    fn duplicate_records_do_not_change_ranked_decay_result() {
        let dependency = record("dep-001", 0.60);

        let once = aggregate_unique_dependencies_with_model(
            &[dependency.clone()],
            DependencyAggregationModel::RankedDecay,
        );

        let duplicated = vec![dependency; 100];

        let repeated = aggregate_unique_dependencies_with_model(
            &duplicated,
            DependencyAggregationModel::RankedDecay,
        );

        assert!((once - repeated).abs() < 1e-9);
    }
}

#[cfg(test)]
mod structural_dependency_tests {
    use super::*;

    fn weights() -> DependencyQualificationWeights {
        DependencyQualificationWeights {
            blocking_relevance: 0.30,
            failure_propagation: 0.30,
            operational_reliance: 0.25,
            business_path_relevance: 0.15,
        }
    }

    fn qualified(score: f64) -> QualifiedDependency {
        qualify_dependency(
            DependencyEvidence {
                blocking_relevance: score,
                failure_propagation: score,
                operational_reliance: score,
                business_path_relevance: score,
            },
            weights(),
        )
        .unwrap()
    }

    fn record(id: &str, source: &str, target: &str, score: f64) -> DependencyRecord {
        DependencyRecord {
            dependency_id: DependencyId(id.to_string()),
            dependency_path_id: DependencyPathId("default-path".to_string()),
            failure_domain_id: FailureDomainId("default-failure-domain".to_string()),
            source_capability: CapabilityId(source.to_string()),
            target_capability: CapabilityId(target.to_string()),
            relationship_type: DependencyRelationshipType::Runtime,
            qualified: qualified(score),
        }
    }

    #[test]
    fn different_ids_same_structure_collapse_to_one_relation() {
        let records = vec![
            record("dep-001", "a", "b", 0.60),
            record("dep-002", "a", "b", 0.60),
            record("dep-003", "a", "b", 0.60),
        ];

        let unique = deduplicate_structural_dependencies(&records);

        assert_eq!(unique.len(), 1);
    }

    #[test]
    fn distinct_structures_are_preserved() {
        let records = vec![
            record("dep-001", "a", "b", 0.60),
            record("dep-002", "a", "c", 0.60),
            record("dep-003", "b", "c", 0.60),
        ];

        let unique = deduplicate_structural_dependencies(&records);

        assert_eq!(unique.len(), 3);
    }

    #[test]
    fn strongest_evidence_wins_for_same_structural_relation() {
        let records = vec![
            record("dep-weak", "a", "b", 0.20),
            record("dep-strong", "a", "b", 0.80),
        ];

        let unique = deduplicate_structural_dependencies(&records);

        assert_eq!(unique.len(), 1);
        assert!((unique[0].qualified.score - 0.80).abs() < 1e-9);
    }
}

#[cfg(test)]
mod dependency_path_tests {
    use super::*;

    fn weights() -> DependencyQualificationWeights {
        DependencyQualificationWeights {
            blocking_relevance: 0.30,
            failure_propagation: 0.30,
            operational_reliance: 0.25,
            business_path_relevance: 0.15,
        }
    }

    fn qualified(score: f64) -> QualifiedDependency {
        qualify_dependency(
            DependencyEvidence {
                blocking_relevance: score,
                failure_propagation: score,
                operational_reliance: score,
                business_path_relevance: score,
            },
            weights(),
        )
        .unwrap()
    }

    fn record(
        id: &str,
        path: &str,
        relation: DependencyRelationshipType,
        score: f64,
    ) -> DependencyRecord {
        DependencyRecord {
            dependency_id: DependencyId(id.to_string()),
            dependency_path_id: DependencyPathId(path.to_string()),
            failure_domain_id: FailureDomainId("default-failure-domain".to_string()),
            source_capability: CapabilityId("capability-a".to_string()),
            target_capability: CapabilityId("capability-b".to_string()),
            relationship_type: relation,
            qualified: qualified(score),
        }
    }

    #[test]
    fn multiple_relation_types_on_same_path_collapse() {
        let records = vec![
            record(
                "runtime",
                "path-a-b",
                DependencyRelationshipType::Runtime,
                0.60,
            ),
            record(
                "data",
                "path-a-b",
                DependencyRelationshipType::DataFlow,
                0.60,
            ),
        ];

        let unique = deduplicate_dependency_paths(&records);

        assert_eq!(unique.len(), 1);
    }

    #[test]
    fn independent_paths_are_preserved() {
        let records = vec![
            record(
                "runtime",
                "path-primary",
                DependencyRelationshipType::Runtime,
                0.60,
            ),
            record(
                "data",
                "path-secondary",
                DependencyRelationshipType::DataFlow,
                0.60,
            ),
        ];

        let unique = deduplicate_dependency_paths(&records);

        assert_eq!(unique.len(), 2);
    }

    #[test]
    fn strongest_observation_represents_same_path() {
        let records = vec![
            record(
                "weak",
                "path-a-b",
                DependencyRelationshipType::Runtime,
                0.40,
            ),
            record(
                "strong",
                "path-a-b",
                DependencyRelationshipType::DataFlow,
                0.80,
            ),
        ];

        let unique = deduplicate_dependency_paths(&records);

        assert_eq!(unique.len(), 1);
        assert!((unique[0].qualified.score - 0.80).abs() < 1e-9);
    }
}
