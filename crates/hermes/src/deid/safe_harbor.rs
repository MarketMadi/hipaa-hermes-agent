//! HIPAA Safe Harbor–oriented identifier rules (45 CFR § 164.514(b)(2)(i)(A–R)).
//! Not legal certification — automated scrubbing aligned to the 18 categories.

use lazy_static::lazy_static;
use regex::Regex;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SafeHarborCategory {
    Name,
    Geography,
    Date,
    Phone,
    Fax,
    Email,
    Ssn,
    Mrn,
    HealthPlanId,
    AccountNumber,
    LicenseNumber,
    VehicleId,
    DeviceId,
    Url,
    Ip,
    Biometric,
    Photo,
    OtherUniqueId,
}

impl SafeHarborCategory {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Geography => "geography",
            Self::Date => "date",
            Self::Phone => "phone",
            Self::Fax => "fax",
            Self::Email => "email",
            Self::Ssn => "ssn",
            Self::Mrn => "mrn",
            Self::HealthPlanId => "health_plan_id",
            Self::AccountNumber => "account_number",
            Self::LicenseNumber => "license_number",
            Self::VehicleId => "vehicle_id",
            Self::DeviceId => "device_id",
            Self::Url => "url",
            Self::Ip => "ip",
            Self::Biometric => "biometric",
            Self::Photo => "photo",
            Self::OtherUniqueId => "other_unique_id",
        }
    }

    pub fn all() -> &'static [SafeHarborCategory] {
        &[
            Self::Name,
            Self::Geography,
            Self::Date,
            Self::Phone,
            Self::Fax,
            Self::Email,
            Self::Ssn,
            Self::Mrn,
            Self::HealthPlanId,
            Self::AccountNumber,
            Self::LicenseNumber,
            Self::VehicleId,
            Self::DeviceId,
            Self::Url,
            Self::Ip,
            Self::Biometric,
            Self::Photo,
            Self::OtherUniqueId,
        ]
    }
}

struct RedactionRule {
    category: SafeHarborCategory,
    pattern: Regex,
    replacement: &'static str,
}

lazy_static! {
    static ref RULES: Vec<RedactionRule> = vec![
        // (A) Names — patient, clinician, relatives
        RedactionRule {
            category: SafeHarborCategory::Name,
            pattern: Regex::new(
                r"\b(?:[Pp]atient|[Mm]r\.?|[Mm]rs\.?|[Mm]s\.?|[Dd]r\.?|[Mm]other|[Ff]ather|[Ss]pouse|[Ss]on|[Dd]aughter|[Bb]rother|[Ss]ister)\s+[A-Z][a-z]+(?:\s+[A-Z][a-z]+){0,2}\b",
            )
            .unwrap(),
            replacement: "[REDACTED-NAME]",
        },
        RedactionRule {
            category: SafeHarborCategory::Name,
            pattern: Regex::new(r"\b[A-Z][a-z]+,\s*(?:MD|DO|RN|NP|PA|PharmD|DDS)\b").unwrap(),
            replacement: "[REDACTED-NAME]",
        },
        // (B) Geography
        RedactionRule {
            category: SafeHarborCategory::Geography,
            pattern: Regex::new(
                r"\b\d{1,5}\s+[A-Za-z0-9]+\s+(?:St|Street|Ave|Avenue|Rd|Road|Blvd|Boulevard|Drive|Dr|Lane|Ln|Way|Court|Ct)\.?\b",
            )
            .unwrap(),
            replacement: "[REDACTED-ADDRESS]",
        },
        RedactionRule {
            category: SafeHarborCategory::Geography,
            pattern: Regex::new(r"\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+)?,\s*[A-Z]{2}\b").unwrap(),
            replacement: "[REDACTED-CITY]",
        },
        RedactionRule {
            category: SafeHarborCategory::Geography,
            pattern: Regex::new(r"\b\d{5}(?:-\d{4})?\b").unwrap(),
            replacement: "[REDACTED-ZIP]",
        },
        // (C) Dates (month/day redacted; year kept per Safe Harbor) + ages >89
        RedactionRule {
            category: SafeHarborCategory::Date,
            pattern: Regex::new(
                r"\b(?:\d{1,2}[/-]\d{1,2}[/-])(\d{2,4})\b",
            )
            .unwrap(),
            replacement: "[REDACTED-DATE]-$1",
        },
        RedactionRule {
            category: SafeHarborCategory::Date,
            pattern: Regex::new(
                r"\b(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\.?\s+\d{1,2},?\s+(\d{4})\b",
            )
            .unwrap(),
            replacement: "[REDACTED-DATE]-$1",
        },
        RedactionRule {
            category: SafeHarborCategory::Date,
            pattern: Regex::new(r"\b\d{4}-\d{2}-\d{2}\b").unwrap(),
            replacement: "[REDACTED-DATE]",
        },
        RedactionRule {
            category: SafeHarborCategory::Date,
            pattern: Regex::new(
                r"(?i)\b(?:age\s*[:=]?\s*(?:9\d|[1-9]\d{2,})|(?:9\d|[1-9]\d{2,})\s*(?:year|yr)s?\s*old)\b",
            )
            .unwrap(),
            replacement: "[AGE-90+]",
        },
        RedactionRule {
            category: SafeHarborCategory::Date,
            pattern: Regex::new(r"(?i)\b(?:age\s*[:=]?\s*\d{1,3}|\d{1,3}\s*(?:year|yr)s?\s*old|\d{1,3}\s*y/?o)\b").unwrap(),
            replacement: "[REDACTED-AGE]",
        },
        // (D–E) Fax before phone — fax numbers also match phone pattern
        RedactionRule {
            category: SafeHarborCategory::Fax,
            pattern: Regex::new(
                r"(?i)\b(?:fax|facsimile)\s*[:#]?\s*(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b",
            )
            .unwrap(),
            replacement: "[REDACTED-FAX]",
        },
        RedactionRule {
            category: SafeHarborCategory::Phone,
            pattern: Regex::new(r"\b(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b").unwrap(),
            replacement: "[REDACTED-PHONE]",
        },
        // (F) Email
        RedactionRule {
            category: SafeHarborCategory::Email,
            pattern: Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap(),
            replacement: "[REDACTED-EMAIL]",
        },
        // (G) SSN
        RedactionRule {
            category: SafeHarborCategory::Ssn,
            pattern: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
            replacement: "[REDACTED-SSN]",
        },
        // (H) MRN
        RedactionRule {
            category: SafeHarborCategory::Mrn,
            pattern: Regex::new(
                r"(?i)\b(?:MRN|medical record(?: number)?|patient id|study id)\s*[:#]?\s*#?[\w-]+\b",
            )
            .unwrap(),
            replacement: "[REDACTED-MRN]",
        },
        // (I) Health plan beneficiary numbers
        RedactionRule {
            category: SafeHarborCategory::HealthPlanId,
            pattern: Regex::new(
                r"(?i)\b(?:member id|subscriber id|beneficiary(?: id)?|policy(?: number)?|health plan id)\s*[:#]?\s*[\w-]+\b",
            )
            .unwrap(),
            replacement: "[REDACTED-PLAN-ID]",
        },
        // (J) Account numbers
        RedactionRule {
            category: SafeHarborCategory::AccountNumber,
            pattern: Regex::new(r"(?i)\b(?:account|acct)(?:\s+number)?\s*[:#]?\s*[\w-]+\b").unwrap(),
            replacement: "[REDACTED-ACCOUNT]",
        },
        // (K) Certificate / license numbers
        RedactionRule {
            category: SafeHarborCategory::LicenseNumber,
            pattern: Regex::new(
                r"(?i)\b(?:license|lic(?:ense)?|certificate|cert)(?:\s+number)?\s*[:#]?\s*[\w-]+\b",
            )
            .unwrap(),
            replacement: "[REDACTED-LICENSE]",
        },
        // (L) Vehicle identifiers
        RedactionRule {
            category: SafeHarborCategory::VehicleId,
            pattern: Regex::new(
                r"(?i)\b(?:vin|vehicle id|license plate|plate number)\s*[:#]?\s*[\w-]+\b",
            )
            .unwrap(),
            replacement: "[REDACTED-VEHICLE]",
        },
        // (M) Device identifiers
        RedactionRule {
            category: SafeHarborCategory::DeviceId,
            pattern: Regex::new(
                r"(?i)\b(?:device id|device serial|serial number|implant id|udi)\s*[:#]?\s*[\w-]+\b",
            )
            .unwrap(),
            replacement: "[REDACTED-DEVICE]",
        },
        // (N) URLs
        RedactionRule {
            category: SafeHarborCategory::Url,
            pattern: Regex::new(r"https?://[^\s]+").unwrap(),
            replacement: "[REDACTED-URL]",
        },
        // (O) IP addresses
        RedactionRule {
            category: SafeHarborCategory::Ip,
            pattern: Regex::new(r"\b\d{1,3}(?:\.\d{1,3}){3}\b").unwrap(),
            replacement: "[REDACTED-IP]",
        },
        // (P) Biometric identifiers
        RedactionRule {
            category: SafeHarborCategory::Biometric,
            pattern: Regex::new(r"(?i)\b(?:fingerprint|voice print|voiceprint|retina scan|biometric)\b").unwrap(),
            replacement: "[REDACTED-BIOMETRIC]",
        },
        // (Q) Full-face photos (text references)
        RedactionRule {
            category: SafeHarborCategory::Photo,
            pattern: Regex::new(r"(?i)\b(?:patient photo|facial image|headshot|portrait photo)\b").unwrap(),
            replacement: "[REDACTED-PHOTO]",
        },
        // (R) Other unique identifying numbers/codes
        RedactionRule {
            category: SafeHarborCategory::OtherUniqueId,
            pattern: Regex::new(r"(?i)\b(?:encounter id|visit id|order id|accession)\s*[:#]?\s*#?[\w-]+\b").unwrap(),
            replacement: "[REDACTED-ID]",
        },
        RedactionRule {
            category: SafeHarborCategory::OtherUniqueId,
            pattern: Regex::new(r"#\d{4,}\b").unwrap(),
            replacement: "[REDACTED-ID]",
        },
        // Sex/gender when paired with identifier context (quasi-identifier)
        RedactionRule {
            category: SafeHarborCategory::OtherUniqueId,
            pattern: Regex::new(r"(?i)\b(?:sex|gender)\s*[:=]\s*[MFNB]\b").unwrap(),
            replacement: "[REDACTED-SEX]",
        },
    ];
}

pub struct RulePassResult {
    pub text: String,
    pub redaction_count: u32,
    pub categories: BTreeSet<SafeHarborCategory>,
}

pub fn apply_safe_harbor_rules(input: &str) -> RulePassResult {
    let mut text = input.to_string();
    let mut redaction_count = 0u32;
    let mut categories = BTreeSet::new();

    for _ in 0..3 {
        for rule in RULES.iter() {
            let count = rule.pattern.find_iter(&text).count();
            if count > 0 {
                text = rule
                    .pattern
                    .replace_all(&text, rule.replacement)
                    .into_owned();
                redaction_count += count as u32;
                categories.insert(rule.category);
            }
        }
    }

    RulePassResult {
        text,
        redaction_count,
        categories,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_categories_have_slug() {
        for cat in SafeHarborCategory::all() {
            assert!(!cat.slug().is_empty());
        }
    }
}
