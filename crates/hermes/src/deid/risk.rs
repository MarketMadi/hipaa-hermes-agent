use lazy_static::lazy_static;
use regex::Regex;

use super::safe_harbor::SafeHarborCategory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

impl RiskLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

lazy_static! {
    static ref POSSIBLE_SSN: Regex = Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap();
    static ref POSSIBLE_PHONE: Regex =
        Regex::new(r"\b\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b").unwrap();
    static ref POSSIBLE_EMAIL: Regex =
        Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap();
    static ref POSSIBLE_NAME: Regex =
        Regex::new(r"\b[A-Z][a-z]{2,}\s+[A-Z][a-z]{2,}\b").unwrap();
    static ref CLINICAL_ALLOWLIST: Regex = Regex::new(
        r"(?i)^(DE-IDENTIFIED|Admission|Discharge|Hospital|Follow-up|Medications|Creatinine|Potassium|BUN|amoxicillin|lisinopril|spironolactone|pneumonia|albuterol|CONTEXT|QUESTION|Practice case)$"
    )
    .unwrap();
}

/// Heuristic scan for identifiers that may remain after rule pass.
pub fn assess_residual_risk(text: &str, categories_hit: &[SafeHarborCategory]) -> (RiskLevel, Vec<String>) {
    let mut warnings = Vec::new();

    if POSSIBLE_SSN.is_match(text) {
        warnings.push("possible_ssn_remainder".into());
    }
    if POSSIBLE_PHONE.is_match(text) {
        warnings.push("possible_phone_remainder".into());
    }
    if POSSIBLE_EMAIL.is_match(text) {
        warnings.push("possible_email_remainder".into());
    }

    for cap in POSSIBLE_NAME.find_iter(text) {
        let m = cap.as_str();
        if m.contains("REDACTED") {
            continue;
        }
        let parts: Vec<&str> = m.split_whitespace().collect();
        if parts.len() == 2
            && (parts[0].chars().all(|c| c.is_ascii_uppercase())
                || CLINICAL_ALLOWLIST.is_match(parts[0])
                || CLINICAL_ALLOWLIST.is_match(parts[1]))
        {
            continue;
        }
        if !m.starts_with("Study ID") {
            warnings.push(format!("possible_name_remainder:{m}"));
            break;
        }
    }

    if categories_hit.is_empty() && text.len() > 200 {
        warnings.push("no_redactions_on_long_text".into());
    }

    let level = if warnings.iter().any(|w| w.starts_with("possible_ssn")) {
        RiskLevel::High
    } else if warnings
        .iter()
        .any(|w| w.starts_with("possible_name") || w.starts_with("possible_phone"))
    {
        RiskLevel::Medium
    } else if warnings.is_empty() {
        RiskLevel::Low
    } else {
        RiskLevel::Medium
    };

    (level, warnings)
}
