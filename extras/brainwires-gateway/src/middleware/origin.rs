//! WebSocket origin validation middleware.
//!
//! Validates the `Origin` header on WebSocket upgrade requests to prevent
//! cross-site WebSocket hijacking (CVE-2026-25253 style attacks).

/// Validates WebSocket connection origins against a configurable allow-list.
///
/// When no origins are configured (empty list), all origins are accepted,
/// which is suitable for development environments. In production, origins
/// should be explicitly listed.
pub struct OriginValidator {
    allowed_origins: Vec<String>,
}

impl OriginValidator {
    /// Create a new `OriginValidator` with the given allowed origins.
    ///
    /// If `allowed_origins` is empty, all origins are allowed (dev mode).
    /// Supports wildcard patterns like `*.example.com`.
    pub fn new(allowed_origins: Vec<String>) -> Self {
        Self { allowed_origins }
    }

    /// Returns `true` if the given origin is allowed.
    ///
    /// - If no allowed origins are configured, all origins pass (dev mode).
    /// - If `origin` is `None`, it is rejected when an allow-list is configured.
    /// - Wildcard patterns like `*.example.com` match any subdomain.
    pub fn validate(&self, origin: Option<&str>) -> bool {
        // Dev mode: no restrictions
        if self.allowed_origins.is_empty() {
            return true;
        }

        let origin = match origin {
            Some(o) => o,
            None => return false,
        };

        for allowed in &self.allowed_origins {
            if allowed.starts_with("*.") {
                // Wildcard: *.example.com matches foo.example.com, bar.example.com
                let suffix = &allowed[1..]; // ".example.com"
                if origin.ends_with(suffix) || origin == &allowed[2..] {
                    return true;
                }
            } else if allowed == origin {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_allowlist_permits_all() {
        let v = OriginValidator::new(vec![]);
        assert!(v.validate(Some("https://evil.com")));
        assert!(v.validate(None));
    }

    #[test]
    fn exact_match_allowed() {
        let v = OriginValidator::new(vec!["https://app.example.com".to_string()]);
        assert!(v.validate(Some("https://app.example.com")));
    }

    #[test]
    fn exact_match_rejected() {
        let v = OriginValidator::new(vec!["https://app.example.com".to_string()]);
        assert!(!v.validate(Some("https://evil.com")));
    }

    #[test]
    fn wildcard_subdomain_match() {
        let v = OriginValidator::new(vec!["*.example.com".to_string()]);
        assert!(v.validate(Some("https://app.example.com")));
        assert!(v.validate(Some("https://staging.example.com")));
    }

    #[test]
    fn wildcard_bare_domain_match() {
        let v = OriginValidator::new(vec!["*.example.com".to_string()]);
        assert!(v.validate(Some("example.com")));
    }

    #[test]
    fn wildcard_rejects_unrelated() {
        let v = OriginValidator::new(vec!["*.example.com".to_string()]);
        assert!(!v.validate(Some("https://evil.com")));
    }

    #[test]
    fn none_origin_rejected_when_allowlist_set() {
        let v = OriginValidator::new(vec!["https://app.example.com".to_string()]);
        assert!(!v.validate(None));
    }

    #[test]
    fn multiple_allowed_origins() {
        let v = OriginValidator::new(vec![
            "https://app.example.com".to_string(),
            "https://dashboard.example.com".to_string(),
        ]);
        assert!(v.validate(Some("https://app.example.com")));
        assert!(v.validate(Some("https://dashboard.example.com")));
        assert!(!v.validate(Some("https://evil.com")));
    }
}
