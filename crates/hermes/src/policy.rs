use lazy_static::lazy_static;
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyResult {
    Allow,
    Deny { reason: &'static str },
}

const ALLOWED_SKILLS: &[&str] = &["vault-answer"];

lazy_static! {
    /// Hard-block patterns on raw input — request rejected, not redacted.
    /// SSN/email/phone in the inbound request are refused (separation from de-id path).
    static ref HARD_BLOCK_SSN: Regex =
        Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("ssn regex");
    static ref HARD_BLOCK_EMAIL: Regex =
        Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").expect("email regex");
    static ref HARD_BLOCK_PHONE: Regex =
        Regex::new(r"\b(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b").expect("phone regex");
}

pub fn check_skill(skill: &str) -> PolicyResult {
    if !ALLOWED_SKILLS.contains(&skill) {
        return PolicyResult::Deny {
            reason: "skill_not_allowed",
        };
    }
    PolicyResult::Allow
}

/// Hard block on raw prompt before de-identification.
pub fn check_hard_block(prompt: &str) -> PolicyResult {
    if HARD_BLOCK_SSN.is_match(prompt) {
        return PolicyResult::Deny {
            reason: "phi_pattern_detected",
        };
    }
    if HARD_BLOCK_EMAIL.is_match(prompt) {
        return PolicyResult::Deny {
            reason: "phi_pattern_detected",
        };
    }
    if HARD_BLOCK_PHONE.is_match(prompt) {
        return PolicyResult::Deny {
            reason: "phi_pattern_detected",
        };
    }
    PolicyResult::Allow
}

/// Legacy combined check for tests.
pub fn check(prompt: &str, skill: &str) -> PolicyResult {
    match check_skill(skill) {
        PolicyResult::Deny { reason } => PolicyResult::Deny { reason },
        PolicyResult::Allow => check_hard_block(prompt),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_ssn() {
        assert_eq!(
            check_hard_block("patient SSN 123-45-6789 note"),
            PolicyResult::Deny {
                reason: "phi_pattern_detected"
            }
        );
    }

    #[test]
    fn allows_deidentified_discharge_without_ssn() {
        assert_eq!(
            check("Age: 67 | Sex: F | MRN: REDACTED", "vault-answer"),
            PolicyResult::Allow
        );
    }

    #[test]
    fn blocks_unknown_skill() {
        assert_eq!(
            check_skill("unknown-skill"),
            PolicyResult::Deny {
                reason: "skill_not_allowed"
            }
        );
    }
}
