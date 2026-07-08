//! De-identification v3 — Safe Harbor–oriented rules + optional NER hybrid (Presidio).

mod presidio;
mod risk;
mod safe_harbor;

use safe_harbor::apply_safe_harbor_rules;

pub use risk::RiskLevel;
pub use safe_harbor::SafeHarborCategory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeidMode {
    Rules,
    Hybrid,
}

impl DeidMode {
    pub fn from_env() -> Self {
        match std::env::var("DEID_MODE")
            .unwrap_or_else(|_| "rules".into())
            .to_lowercase()
            .as_str()
        {
            "hybrid" => Self::Hybrid,
            _ => Self::Rules,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeidConfig {
    pub mode: DeidMode,
    pub ner_url: String,
    pub block_on_high_risk: bool,
}

impl DeidConfig {
    pub fn from_env() -> Self {
        let block_on_high_risk = match std::env::var("DEID_BLOCK_ON_HIGH_RISK").as_deref() {
            Ok("1") | Ok("true") => true,
            _ => false,
        };
        Self {
            mode: DeidMode::from_env(),
            ner_url: std::env::var("DEID_NER_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:3001".into()),
            block_on_high_risk,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeidResult {
    pub text: String,
    pub redaction_count: u32,
    /// Category slugs for API / audit (e.g. "name", "ssn").
    pub categories: Vec<String>,
    pub safe_harbor_categories: Vec<SafeHarborCategory>,
    pub residual_risk: RiskLevel,
    pub validation_warnings: Vec<String>,
}

impl DeidResult {
    fn from_parts(
        text: String,
        redaction_count: u32,
        categories: std::collections::BTreeSet<SafeHarborCategory>,
        residual_risk: RiskLevel,
        validation_warnings: Vec<String>,
    ) -> Self {
        let safe_harbor_categories: Vec<_> = categories.iter().copied().collect();
        let categories: Vec<String> = safe_harbor_categories.iter().map(|c| c.slug().to_string()).collect();
        Self {
            text,
            redaction_count,
            categories,
            safe_harbor_categories,
            residual_risk,
            validation_warnings,
        }
    }
}

/// Synchronous scrub — rules only (used in tests and as fallback).
pub fn scrub(input: &str) -> DeidResult {
    scrub_with_config(input, &DeidConfig::from_env())
}

pub fn scrub_with_config(input: &str, config: &DeidConfig) -> DeidResult {
    let rules = apply_safe_harbor_rules(input);
    let mut text = rules.text;
    let mut redaction_count = rules.redaction_count;
    let mut categories = rules.categories;

    if config.mode == DeidMode::Hybrid {
        if let Ok(ner) = presidio::apply_presidio_redactions(&text, &config.ner_url) {
            text = ner.text;
            redaction_count += ner.redaction_count;
            categories.extend(ner.categories);
        }
    }

    let cats_vec: Vec<_> = categories.iter().copied().collect();
    let (residual_risk, validation_warnings) = risk::assess_residual_risk(&text, &cats_vec);

    DeidResult::from_parts(text, redaction_count, categories, residual_risk, validation_warnings)
}

pub async fn scrub_async(input: &str, config: &DeidConfig) -> DeidResult {
    let input = input.to_string();
    let config = config.clone();
    tokio::task::spawn_blocking(move || scrub_with_config(&input, &config))
        .await
        .unwrap_or_else(|_| scrub(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrubs_age_and_mrn_from_discharge_note() {
        let input = "DE-IDENTIFIED DISCHARGE NOTE\nAge: 67 | Sex: F | MRN: ABC123\nAdmission: pneumonia.";
        let result = scrub(input);
        assert!(result.text.contains("[REDACTED-AGE]"));
        assert!(result.text.contains("[REDACTED-MRN]"));
        assert!(!result.text.contains("Age: 67"));
        assert!(result.redaction_count >= 2);
    }

    #[test]
    fn scrubs_ssn() {
        let result = scrub("SSN 123-45-6789 on file");
        assert!(result.text.contains("[REDACTED-SSN]"));
        assert!(!result.text.contains("123-45-6789"));
    }

    #[test]
    fn scrubs_patient_name() {
        let result = scrub("Patient John Doe presented with cough");
        assert!(result.text.contains("[REDACTED-NAME]"));
        assert!(!result.text.contains("John Doe"));
    }

    #[test]
    fn does_not_redact_patient_and_family() {
        let result = scrub("summary for the patient and family");
        assert!(!result.text.contains("[REDACTED-NAME]"));
        assert!(result.text.contains("patient and family"));
    }

    #[test]
    fn buckets_age_over_89() {
        let result = scrub("Age: 91 years old, otherwise stable");
        assert!(result.text.contains("[AGE-90+]"));
    }

    #[test]
    fn keeps_year_on_date_redaction() {
        let result = scrub("admitted 03/15/2024");
        assert!(result.text.contains("2024"));
        assert!(result.text.contains("[REDACTED-DATE]"));
    }
}
