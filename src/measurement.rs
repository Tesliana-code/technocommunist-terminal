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
