use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeStatus { Experimental, Unsupported }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleReviewStatus { BehaviorOnly, MissingRules }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Warning {
    pub code: String,
    pub message: String,
}
impl Warning {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self { code: code.into(), message: message.into() }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalculationOutcome<T> {
    pub status: OutcomeStatus,
    pub rule_review_status: RuleReviewStatus,
    pub policy_version: String,
    /// None is intentional for unsupported calculations; never a zero/fake result.
    pub data: Option<T>,
    pub warnings: Vec<Warning>,
    pub evidence_refs: Vec<String>,
}
impl<T> CalculationOutcome<T> {
    pub(crate) fn experimental(data: T, policy: &str, evidence: &[&str], explanation: &str) -> Self {
        Self {
            status: OutcomeStatus::Experimental,
            rule_review_status: RuleReviewStatus::BehaviorOnly,
            policy_version: policy.to_owned(),
            data: Some(data),
            warnings: vec![Warning::new("behavior_only", "Reprodução experimental de observações limitadas; não é validação normativa."),
                Warning::new("scope_limit", explanation)],
            evidence_refs: evidence.iter().map(|v| (*v).to_owned()).collect(),
        }
    }
    pub(crate) fn unsupported(policy: &str, mut warnings: Vec<Warning>) -> Self {
        warnings.push(Warning::new("missing_rules", "Regras, índices e enquadramento não recuperados. Não há cálculo de direito, RMI ou CTC."));
        Self { status: OutcomeStatus::Unsupported, rule_review_status: RuleReviewStatus::MissingRules,
            policy_version: policy.to_owned(), data: None, warnings, evidence_refs: vec![] }
    }
}
