//! Rule-based de-identification (HIPAA Safe Harbor–oriented heuristics).
//! v2: scrub before LLM; original prompt is never sent to the vendor.

use lazy_static::lazy_static;
use regex::Regex;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeidResult {
    pub text: String,
    pub redaction_count: u32,
    pub categories: Vec<String>,
}

struct RedactionRule {
    category: &'static str,
    pattern: Regex,
    replacement: &'static str,
}

lazy_static! {
    static ref RULES: Vec<RedactionRule> = vec![
        RedactionRule {
            category: "ssn",
            pattern: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
            replacement: "[REDACTED-SSN]",
        },
        RedactionRule {
            category: "email",
            pattern: Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap(),
            replacement: "[REDACTED-EMAIL]",
        },
        RedactionRule {
            category: "phone",
            pattern: Regex::new(r"\b(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b").unwrap(),
            replacement: "[REDACTED-PHONE]",
        },
        RedactionRule {
            category: "mrn",
            pattern: Regex::new(r"(?i)\b(?:MRN|medical record(?: number)?|patient id|study id)\s*[:#]?\s*#?[\w-]+\b").unwrap(),
            replacement: "[REDACTED-MRN]",
        },
        RedactionRule {
            category: "date",
            pattern: Regex::new(
                r"\b(?:\d{1,2}[/-]\d{1,2}[/-]\d{2,4}|\d{4}-\d{2}-\d{2}|(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\.?\s+\d{1,2},?\s+\d{4})\b",
            )
            .unwrap(),
            replacement: "[REDACTED-DATE]",
        },
        RedactionRule {
            category: "age",
            pattern: Regex::new(r"(?i)\b(?:age\s*[:=]?\s*\d{1,3}|\d{1,3}\s*(?:year|yr)s?\s*old|\d{1,3}\s*y/?o)\b").unwrap(),
            replacement: "[REDACTED-AGE]",
        },
        RedactionRule {
            category: "name",
            pattern: Regex::new(
                r"\b(?:[Pp]atient|[Mm]r\.?|[Mm]rs\.?|[Mm]s\.?|[Dd]r\.?)\s+[A-Z][a-z]+(?:\s+[A-Z][a-z]+){0,2}\b",
            )
            .unwrap(),
            replacement: "[REDACTED-NAME]",
        },
        RedactionRule {
            category: "address",
            pattern: Regex::new(r"\b\d{1,5}\s+[A-Za-z0-9]+\s+(?:St|Street|Ave|Avenue|Rd|Road|Blvd|Drive|Dr|Lane|Ln)\.?\b").unwrap(),
            replacement: "[REDACTED-ADDRESS]",
        },
        RedactionRule {
            category: "zip",
            pattern: Regex::new(r"\b\d{5}(?:-\d{4})?\b").unwrap(),
            replacement: "[REDACTED-ZIP]",
        },
        RedactionRule {
            category: "ip",
            pattern: Regex::new(r"\b\d{1,3}(?:\.\d{1,3}){3}\b").unwrap(),
            replacement: "[REDACTED-IP]",
        },
        RedactionRule {
            category: "url",
            pattern: Regex::new(r"https?://[^\s]+").unwrap(),
            replacement: "[REDACTED-URL]",
        },
        RedactionRule {
            category: "sex",
            pattern: Regex::new(r"(?i)\b(?:sex|gender)\s*[:=]\s*[MFNB]\b").unwrap(),
            replacement: "[REDACTED-SEX]",
        },
    ];
}

/// Scrub HIPAA-oriented identifiers. Run on allowed prompts before LLM.
pub fn scrub(input: &str) -> DeidResult {
    let mut text = input.to_string();
    let mut redaction_count = 0u32;
    let mut categories = BTreeSet::new();

    // Multiple passes — redactions can expose new edges
    for _ in 0..3 {
        for rule in RULES.iter() {
            let count = rule.pattern.find_iter(&text).count();
            if count > 0 {
                text = rule.pattern.replace_all(&text, rule.replacement).into_owned();
                redaction_count += count as u32;
                categories.insert(rule.category.to_string());
            }
        }
    }

    DeidResult {
        text,
        redaction_count,
        categories: categories.into_iter().collect(),
    }
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
        assert!(result.text.contains("[REDACTED-SEX]"));
        assert!(!result.text.contains("Age: 67"));
        assert!(result.redaction_count >= 3);
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
}
