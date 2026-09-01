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
