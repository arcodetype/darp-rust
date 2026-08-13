//! Validation and normalization for service URL aliases (`urls`).
//!
//! An alias is a *hostname*, not a URL. Whatever a config supplies here is
//! interpolated straight into an nginx `server_name`, a hosts-file line, and
//! `portmap.json`, so a value carrying a scheme, port, path, or an nginx
//! delimiter can produce an unusable mapping or break the shared reverse-proxy
//! config for every other service. Everything in this module is pure — no file
//! I/O, no printing, no deployment state — so `cmd_deploy` can validate a whole
//! deployment before it truncates or rewrites anything.

use std::fmt;

/// Maximum total length of a DNS name, per RFC 1035.
const MAX_HOSTNAME_LEN: usize = 253;
/// Maximum length of a single DNS label, per RFC 1035.
const MAX_LABEL_LEN: usize = 63;

/// Why one configured alias was rejected.
///
/// Split out from [`AliasValidationError`] so the deploy-time reporter can group
/// by service and still print an actionable reason per alias.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasErrorReason {
    Empty,
    Scheme,
    UserInfo,
    Path,
    Query,
    Fragment,
    Port,
    Wildcard,
    IpLiteral,
    NonAscii,
    TrailingDots,
    HostnameTooLong,
    EmptyLabel,
    LabelTooLong,
    InvalidCharacter,
    LeadingHyphen,
    TrailingHyphen,
}

impl AliasErrorReason {
    /// The user-facing explanation, phrased as what the alias must not contain so
    /// it reads correctly under a `"<input>" — <reason>` bullet.
    pub fn message(self) -> &'static str {
        match self {
            Self::Empty => "aliases must not be empty",
            Self::Scheme => "aliases must not include a URL scheme",
            Self::UserInfo => "aliases must not include user information",
            Self::Path => "aliases must not include a path",
            Self::Query => "aliases must not include a query string",
            Self::Fragment => "aliases must not include a fragment",
            Self::Port => "aliases must not include a port",
            Self::Wildcard => "wildcard hostnames are not supported",
            Self::IpLiteral => "aliases must be hostnames, not IP address literals",
            Self::NonAscii => "internationalized hostnames must be supplied in ASCII/Punycode form",
            Self::TrailingDots => "aliases may end with at most one trailing dot",
            Self::HostnameTooLong => "hostnames must not exceed 253 characters",
            Self::EmptyLabel => "hostname labels must not be empty",
            Self::LabelTooLong => "hostname labels must not exceed 63 characters",
            Self::InvalidCharacter => {
                "hostname labels may contain only letters, digits, and hyphens"
            }
            Self::LeadingHyphen => "hostname labels must not begin with a hyphen",
            Self::TrailingHyphen => "hostname labels must not end with a hyphen",
        }
    }
}

impl fmt::Display for AliasErrorReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

/// A rejected alias, carrying the original configured text so the deploy error
/// quotes what the engineer actually wrote rather than a normalized guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasValidationError {
    pub input: String,
    pub reason: AliasErrorReason,
}

impl fmt::Display for AliasValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} — {}", self.input, self.reason)
    }
}

impl std::error::Error for AliasValidationError {}

/// Validate one configured alias and return its normalized hostname.
///
/// Normalization is: trim surrounding whitespace, drop at most one trailing DNS
/// dot, lowercase ASCII letters. The result is what every downstream consumer
/// must use — collision detection, `server_name`, `hosts_container`, the system
/// hosts file, `portmap.json`, and `darp urls` — so two spellings of one name
/// can never diverge between artifacts.
///
/// Wildcards and IP literals are rejected deliberately: this feature names
/// alternate hostnames, not alternate listener addresses or nginx patterns.
pub fn validate_and_normalize_alias(input: &str) -> Result<String, AliasValidationError> {
    let reject = |reason: AliasErrorReason| AliasValidationError {
        input: input.to_string(),
        reason,
    };

    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(reject(AliasErrorReason::Empty));
    }

    // Before any structural check: a raw UTF-8 name needs the Punycode advice, not
    // a confusing complaint about label characters.
    if !trimmed.is_ascii() {
        return Err(reject(AliasErrorReason::NonAscii));
    }

    // URL-shaped input gets a reason naming the specific component, which is far
    // more actionable than the generic label-charset message.
    if trimmed.contains("://") {
        return Err(reject(AliasErrorReason::Scheme));
    }
    if trimmed.contains('*') {
        return Err(reject(AliasErrorReason::Wildcard));
    }
    if trimmed.contains('@') {
        return Err(reject(AliasErrorReason::UserInfo));
    }
    if trimmed.contains('/') {
        return Err(reject(AliasErrorReason::Path));
    }
    if trimmed.contains('?') {
        return Err(reject(AliasErrorReason::Query));
    }
    if trimmed.contains('#') {
        return Err(reject(AliasErrorReason::Fragment));
    }
    // Bracketed IPv6 (`[::1]`, `[::1]:80`) before the colon check, which would
    // otherwise report it as a port problem.
    if trimmed.contains('[') || trimmed.contains(']') {
        return Err(reject(AliasErrorReason::IpLiteral));
    }
    if is_ip_literal(trimmed) {
        return Err(reject(AliasErrorReason::IpLiteral));
    }
    if let Some(idx) = trimmed.find(':') {
        // `host:8080` is a port; `mailto:x` (or any other non-numeric tail) is a
        // scheme that lacked the `//`.
        let tail = &trimmed[idx + 1..];
        let reason = if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) {
            AliasErrorReason::Port
        } else {
            AliasErrorReason::Scheme
        };
        return Err(reject(reason));
    }

    // One trailing dot is the fully-qualified form and is accepted; two means a
    // malformed empty label, which is clearer to name than to silently collapse.
    if trimmed.ends_with("..") {
        return Err(reject(AliasErrorReason::TrailingDots));
    }
    let candidate = trimmed.strip_suffix('.').unwrap_or(trimmed);
    if candidate.is_empty() {
        return Err(reject(AliasErrorReason::Empty));
    }

    let normalized = candidate.to_ascii_lowercase();

    // A dotted-quad also has to be caught after the trailing dot comes off, so
    // `127.0.0.1.` doesn't slip through as a hostname.
    if is_ip_literal(&normalized) {
        return Err(reject(AliasErrorReason::IpLiteral));
    }

    if normalized.len() > MAX_HOSTNAME_LEN {
        return Err(reject(AliasErrorReason::HostnameTooLong));
    }

    for label in normalized.split('.') {
        if label.is_empty() {
            return Err(reject(AliasErrorReason::EmptyLabel));
        }
        if label.len() > MAX_LABEL_LEN {
            return Err(reject(AliasErrorReason::LabelTooLong));
        }
        // Catches whitespace, control characters, newlines, and the nginx/hosts
        // delimiters (`;`, `{`, `}`, quotes, backslash) in one rule.
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(reject(AliasErrorReason::InvalidCharacter));
        }
        if label.starts_with('-') {
            return Err(reject(AliasErrorReason::LeadingHyphen));
        }
        if label.ends_with('-') {
            return Err(reject(AliasErrorReason::TrailingHyphen));
        }
    }

    Ok(normalized)
}

/// True when the value is an IPv4 or IPv6 address literal rather than a hostname.
fn is_ip_literal(value: &str) -> bool {
    value.parse::<std::net::IpAddr>().is_ok()
}

/// Normalize an already-trusted hostname for comparison only — the canonical
/// `{service}.{domain}.test` names, which darp builds from folder names it does
/// not get to validate.
///
/// Comparison has to be DNS-correct (case-insensitive, trailing dot optional)
/// even when the name itself is passed through to nginx verbatim, so an alias
/// spelled `MyApp.comagine.test` is recognized as its own canonical URL.
pub fn normalize_for_comparison(hostname: &str) -> String {
    let trimmed = hostname.trim();
    let stripped = trimmed.strip_suffix('.').unwrap_or(trimmed);
    stripped.to_ascii_lowercase()
}

/// True when the dnsmasq wildcard (`address=/.test/127.0.0.1`) already resolves
/// this hostname, so darp does not need a hosts-file entry to make it reachable.
///
/// Suffix match is case-insensitive but strict: `example.test.invalid` is not a
/// `.test` name.
pub fn is_dnsmasq_wildcard_hostname(hostname: &str) -> bool {
    normalize_for_comparison(hostname).ends_with(".test")
}
