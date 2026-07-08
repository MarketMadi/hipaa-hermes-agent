//! Safe Harbor category fixture tests (45 CFR § 164.514(b)(2)(i)).

use hermes::deid::{scrub, SafeHarborCategory};

struct Fixture {
    category: SafeHarborCategory,
    input: &'static str,
    must_contain: &'static str,
    must_not_contain: &'static str,
}

fn fixtures() -> Vec<Fixture> {
    vec![
        Fixture {
            category: SafeHarborCategory::Name,
            input: "Patient John Doe seen by Dr. Amy Chen",
            must_contain: "[REDACTED-NAME]",
            must_not_contain: "John Doe",
        },
        Fixture {
            category: SafeHarborCategory::Name,
            input: "Attending Smith, MD ordered labs",
            must_contain: "[REDACTED-NAME]",
            must_not_contain: "Smith, MD",
        },
        Fixture {
            category: SafeHarborCategory::Geography,
            input: "Lives at 742 Evergreen Terrace Springfield, IL 62704",
            must_contain: "[REDACTED",
            must_not_contain: "62704",
        },
        Fixture {
            category: SafeHarborCategory::Date,
            input: "admitted 03/15/2024, discharge 03/20/2024",
            must_contain: "[REDACTED-DATE]",
            must_not_contain: "03/15",
        },
        Fixture {
            category: SafeHarborCategory::Date,
            input: "Age: 92 years old",
            must_contain: "[AGE-90+]",
            must_not_contain: "92 years",
        },
        Fixture {
            category: SafeHarborCategory::Phone,
            input: "callback at (555) 867-5309",
            must_contain: "[REDACTED-PHONE]",
            must_not_contain: "867-5309",
        },
        Fixture {
            category: SafeHarborCategory::Fax,
            input: "fax 555-867-5309 for records",
            must_contain: "[REDACTED-FAX]",
            must_not_contain: "867-5309",
        },
        Fixture {
            category: SafeHarborCategory::Email,
            input: "contact jane.doe@clinic.org",
            must_contain: "[REDACTED-EMAIL]",
            must_not_contain: "jane.doe@",
        },
        Fixture {
            category: SafeHarborCategory::Ssn,
            input: "SSN 123-45-6789 verified",
            must_contain: "[REDACTED-SSN]",
            must_not_contain: "123-45-6789",
        },
        Fixture {
            category: SafeHarborCategory::Mrn,
            input: "MRN: ABC-99102 in chart",
            must_contain: "[REDACTED-MRN]",
            must_not_contain: "ABC-99102",
        },
        Fixture {
            category: SafeHarborCategory::HealthPlanId,
            input: "member id: HP-77821",
            must_contain: "[REDACTED-PLAN-ID]",
            must_not_contain: "HP-77821",
        },
        Fixture {
            category: SafeHarborCategory::AccountNumber,
            input: "account number ACCT-4455",
            must_contain: "[REDACTED-ACCOUNT]",
            must_not_contain: "ACCT-4455",
        },
        Fixture {
            category: SafeHarborCategory::LicenseNumber,
            input: "license number MD-123456",
            must_contain: "[REDACTED-LICENSE]",
            must_not_contain: "MD-123456",
        },
        Fixture {
            category: SafeHarborCategory::VehicleId,
            input: "VIN 1HGBH41JXMN109186 noted",
            must_contain: "[REDACTED-VEHICLE]",
            must_not_contain: "1HGBH41JXMN109186",
        },
        Fixture {
            category: SafeHarborCategory::DeviceId,
            input: "device serial SN-998877",
            must_contain: "[REDACTED-DEVICE]",
            must_not_contain: "SN-998877",
        },
        Fixture {
            category: SafeHarborCategory::Url,
            input: "portal https://patient.example.com/records",
            must_contain: "[REDACTED-URL]",
            must_not_contain: "https://",
        },
        Fixture {
            category: SafeHarborCategory::Ip,
            input: "logged from 192.168.1.42",
            must_contain: "[REDACTED-IP]",
            must_not_contain: "192.168.1.42",
        },
        Fixture {
            category: SafeHarborCategory::Biometric,
            input: "fingerprint on file",
            must_contain: "[REDACTED-BIOMETRIC]",
            must_not_contain: "fingerprint",
        },
        Fixture {
            category: SafeHarborCategory::Photo,
            input: "patient photo attached to chart",
            must_contain: "[REDACTED-PHOTO]",
            must_not_contain: "patient photo",
        },
        Fixture {
            category: SafeHarborCategory::OtherUniqueId,
            input: "encounter id ENC-44221",
            must_contain: "[REDACTED-ID]",
            must_not_contain: "ENC-44221",
        },
    ]
}

#[test]
fn safe_harbor_category_fixtures() {
    for fix in fixtures() {
        let result = scrub(fix.input);
        assert!(
            result.text.contains(fix.must_contain),
            "category {:?} expected {:?} in {:?}, got {:?}",
            fix.category,
            fix.must_contain,
            fix.input,
            result.text
        );
        assert!(
            !result.text.contains(fix.must_not_contain),
            "category {:?} should not contain {:?} in {:?}, got {:?}",
            fix.category,
            fix.must_not_contain,
            fix.input,
            result.text
        );
        assert!(
            result.safe_harbor_categories.contains(&fix.category),
            "category {:?} not recorded for {:?}",
            fix.category,
            fix.input
        );
    }
}

#[test]
fn negative_clinical_terms_preserved() {
    let input = "Diagnosis: pneumonia. Meds: amoxicillin, lisinopril. Follow-up with PCP.";
    let result = scrub(input);
    assert!(result.text.contains("pneumonia"));
    assert!(result.text.contains("amoxicillin"));
    assert!(result.text.contains("lisinopril"));
    assert!(!result.text.contains("[REDACTED-NAME]"));
}

#[test]
fn demo_discharge_scenario_scrubs_identifiers() {
    let input = "DE-IDENTIFIED DISCHARGE NOTE\nAge: 67 | Sex: F | MRN: REDACTED\nAdmission: uncomplicated pneumonia.\nDischarge meds: amoxicillin 7d.\nFollow-up: PCP in 1 week.\n\nQUESTION:\nWrite a summary for the patient and family.";
    let result = scrub(input);
    assert!(result.text.contains("[REDACTED-AGE]"));
    assert!(result.text.contains("pneumonia"));
    assert!(result.text.contains("patient and family"));
    assert_eq!(result.residual_risk.as_str(), "low");
}
