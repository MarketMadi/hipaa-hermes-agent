//! Optional Presidio analyzer sidecar for hybrid de-identification (v3.1).

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::time::Duration;

use super::safe_harbor::SafeHarborCategory;

lazy_static! {
    static ref HTTP: ureq::Agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(30))
        .build();
}

#[derive(Debug)]
pub struct PresidioPassResult {
    pub text: String,
    pub redaction_count: u32,
    pub categories: BTreeSet<SafeHarborCategory>,
}

#[derive(Serialize)]
struct AnalyzeRequest<'a> {
    text: &'a str,
    language: &'a str,
}

#[derive(Deserialize)]
struct AnalyzeResponse {
    #[serde(default)]
    entities: Vec<Entity>,
}

#[derive(Deserialize)]
struct Entity {
    entity_type: String,
    start: usize,
    end: usize,
}

fn map_entity_type(entity_type: &str) -> Option<SafeHarborCategory> {
    match entity_type {
        "PERSON" => Some(SafeHarborCategory::Name),
        "EMAIL_ADDRESS" => Some(SafeHarborCategory::Email),
        "PHONE_NUMBER" => Some(SafeHarborCategory::Phone),
        "US_SSN" => Some(SafeHarborCategory::Ssn),
        "DATE_TIME" => Some(SafeHarborCategory::Date),
        "LOCATION" | "US_DRIVER_LICENSE" => Some(SafeHarborCategory::Geography),
        "IP_ADDRESS" => Some(SafeHarborCategory::Ip),
        "URL" => Some(SafeHarborCategory::Url),
        "CREDIT_CARD" | "US_BANK_NUMBER" => Some(SafeHarborCategory::AccountNumber),
        "MEDICAL_LICENSE" | "NRP" => Some(SafeHarborCategory::LicenseNumber),
        _ => Some(SafeHarborCategory::OtherUniqueId),
    }
}

fn replacement_for(category: SafeHarborCategory) -> &'static str {
    match category {
        SafeHarborCategory::Name => "[REDACTED-NAME]",
        SafeHarborCategory::Geography => "[REDACTED-GEO]",
        SafeHarborCategory::Date => "[REDACTED-DATE]",
        SafeHarborCategory::Phone => "[REDACTED-PHONE]",
        SafeHarborCategory::Fax => "[REDACTED-FAX]",
        SafeHarborCategory::Email => "[REDACTED-EMAIL]",
        SafeHarborCategory::Ssn => "[REDACTED-SSN]",
        SafeHarborCategory::Mrn => "[REDACTED-MRN]",
        SafeHarborCategory::HealthPlanId => "[REDACTED-PLAN-ID]",
        SafeHarborCategory::AccountNumber => "[REDACTED-ACCOUNT]",
        SafeHarborCategory::LicenseNumber => "[REDACTED-LICENSE]",
        SafeHarborCategory::VehicleId => "[REDACTED-VEHICLE]",
        SafeHarborCategory::DeviceId => "[REDACTED-DEVICE]",
        SafeHarborCategory::Url => "[REDACTED-URL]",
        SafeHarborCategory::Ip => "[REDACTED-IP]",
        SafeHarborCategory::Biometric => "[REDACTED-BIOMETRIC]",
        SafeHarborCategory::Photo => "[REDACTED-PHOTO]",
        SafeHarborCategory::OtherUniqueId => "[REDACTED-ID]",
    }
}

/// Call Presidio analyzer and redact detected spans.
pub fn apply_presidio_redactions(
    original: &str,
    base_url: &str,
) -> Result<PresidioPassResult, String> {
    let url = format!("{}/analyze", base_url.trim_end_matches('/'));
    let body = AnalyzeRequest {
        text: original,
        language: "en",
    };

    let response = HTTP
        .post(&url)
        .set("Content-Type", "application/json")
        .send_json(&body)
        .map_err(|e| e.to_string())?;

    if response.status() != 200 {
        return Err(format!("presidio status {}", response.status()));
    }

    let parsed: AnalyzeResponse = response.into_json().map_err(|e| e.to_string())?;

    let mut entities: Vec<(usize, usize, SafeHarborCategory)> = parsed
        .entities
        .into_iter()
        .filter_map(|e| {
            let cat = map_entity_type(&e.entity_type)?;
            if e.end > e.start && e.end <= original.len() {
                Some((e.start, e.end, cat))
            } else {
                None
            }
        })
        .collect();

    entities.sort_by(|a, b| b.0.cmp(&a.0));

    let mut text = original.to_string();
    let mut categories = BTreeSet::new();
    let mut redaction_count = 0u32;

    for (start, end, cat) in entities {
        if start < text.len() && end <= text.len() {
            let rep = replacement_for(cat);
            text.replace_range(start..end, rep);
            categories.insert(cat);
            redaction_count += 1;
        }
    }

    Ok(PresidioPassResult {
        text,
        redaction_count,
        categories,
    })
}
