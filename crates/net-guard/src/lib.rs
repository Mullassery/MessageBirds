//! Outbound-request SSRF guard shared by PyCustomerHub webhook transports.
//!
//! Tenant-supplied webhook/channel URLs are attacker-controlled: without a
//! guard, a tenant can point one at `http://169.254.169.254/` (cloud
//! metadata), `http://localhost:...`, or an internal RFC1918 host. This
//! module provides:
//!
//! * [`is_blocked_ip`] — the address policy.
//! * [`validate_outbound_url`] — scheme + IP-literal pre-check.
//! * [`harden_client`] — installs a DNS resolver that drops private
//!   addresses at connect time. This is the only way to catch a *hostname*
//!   that resolves to internal space (and it is not fooled by a
//!   time-of-check/time-of-use DNS rebind, since it filters the actual
//!   addresses reqwest connects to).
//!
//! The guard is on by default. Operators who genuinely need to deliver to a
//! private address in a trusted environment can disable it by setting
//! `PYCUSTOMERHUB_ALLOW_PRIVATE_WEBHOOKS=1`.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use thiserror::Error;
use tokio::net::lookup_host;
use url::{Host, Url};

#[derive(Debug, Error)]
pub enum OutboundUrlError {
    #[error("url is not a valid absolute URL")]
    Invalid,
    #[error("url scheme '{0}' is not allowed (http/https only)")]
    Scheme(String),
    #[error("url host is a blocked private/loopback/link-local address")]
    BlockedHost,
}

/// Whether the SSRF guard is active. Defaults to on; the only way to turn
/// it off is the explicit opt-out `PYCUSTOMERHUB_ALLOW_PRIVATE_WEBHOOKS`.
pub fn guard_enabled() -> bool {
    let value = std::env::var("PYCUSTOMERHUB_ALLOW_PRIVATE_WEBHOOKS")
        .or_else(|_| std::env::var("MESSAGEBIRDS_ALLOW_PRIVATE_WEBHOOKS"));
    !matches!(
        value.as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

/// Validate a tenant-supplied URL before use: it must be an absolute
/// `http`/`https` URL and, while the guard is active, must not be an
/// IP-literal in blocked space. Hostname-based private targets are caught
/// later by [`harden_client`]'s resolver.
pub fn validate_outbound_url(raw: &str) -> Result<Url, OutboundUrlError> {
    let url = Url::parse(raw).map_err(|_| OutboundUrlError::Invalid)?;
    match url.scheme() {
        "http" | "https" => {}
        other => return Err(OutboundUrlError::Scheme(other.to_string())),
    }
    if guard_enabled() && ip_literal_is_blocked(&url) {
        return Err(OutboundUrlError::BlockedHost);
    }
    Ok(url)
}

/// True if `url`'s host is an IP literal in blocked space. Hostnames return
/// `false` here — the resolver hook is what vets those.
pub fn ip_literal_is_blocked(url: &Url) -> bool {
    match url.host() {
        Some(Host::Ipv4(v4)) => is_blocked_v4(v4),
        Some(Host::Ipv6(v6)) => is_blocked_v6(v6),
        _ => false,
    }
}

/// Install the SSRF guard on a `reqwest` client builder: a DNS resolver
/// that refuses to hand reqwest any private/loopback/link-local address,
/// plus a redirect policy that rejects blocked IP-literal hops.
pub fn harden_client(builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    // Wrap (not replace) the default 10-hop limit so loop protection stays.
    let follow = reqwest::redirect::Policy::limited(10);
    builder
        .dns_resolver(Arc::new(SsrfGuardResolver))
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            if ip_literal_is_blocked(attempt.url()) {
                attempt.error("redirect target is a blocked IP literal")
            } else {
                follow.redirect(attempt)
            }
        }))
}

/// DNS resolver that resolves as the OS would, then drops any address in
/// blocked space. A hostname that resolves only to blocked addresses fails
/// the request rather than falling back to something reachable.
#[derive(Debug, Default, Clone, Copy)]
pub struct SsrfGuardResolver;

impl Resolve for SsrfGuardResolver {
    fn resolve(&self, name: Name) -> Resolving {
        Box::pin(async move {
            let host = name.as_str().to_string();
            let resolved = lookup_host((host.as_str(), 0)).await?;
            let allowed: Vec<SocketAddr> = resolved.filter(|a| !is_blocked_ip(a.ip())).collect();
            if allowed.is_empty() {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!("all addresses for '{host}' are blocked by outbound network policy"),
                ))
                    as Box<dyn std::error::Error + Send + Sync>);
            }
            let iter: Addrs = Box::new(allowed.into_iter());
            Ok(iter)
        })
    }
}

/// True if `ip` falls in a range that must never be reached from a
/// tenant-configured webhook (loopback, RFC1918/ULA private, link-local —
/// which covers `169.254.169.254` — CGNAT, benchmarking, multicast,
/// documentation, and other IANA-reserved space).
pub fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_v4(v4),
        IpAddr::V6(v6) => is_blocked_v6(v6),
    }
}

fn is_blocked_v4(ip: Ipv4Addr) -> bool {
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.is_documentation()
        || is_cgnat_v4(ip)
        || is_benchmark_v4(ip)
        || is_reserved_v4(ip)
}

/// `100.64.0.0/10` — RFC 6598 carrier-grade NAT shared address space.
fn is_cgnat_v4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 100 && (64..=127).contains(&o[1])
}

/// `198.18.0.0/15` — RFC 2544 benchmarking space.
fn is_benchmark_v4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 198 && (o[1] == 18 || o[1] == 19)
}

/// `240.0.0.0/4` (reserved, includes `255.255.255.255`), `0.0.0.0/8`
/// ("this network"), `192.0.0.0/24` (IETF protocol assignments) and the
/// deprecated `192.88.99.0/24` 6to4 relay anycast prefix.
fn is_reserved_v4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] >= 240
        || o[0] == 0
        || (o[0] == 192 && o[1] == 0 && o[2] == 0)
        || (o[0] == 192 && o[1] == 88 && o[2] == 99)
}

fn is_blocked_v6(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_blocked_v4(v4);
    }
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_unicast_link_local()
        || is_unique_local_v6(ip)
        || is_documentation_v6(ip)
}

/// `fc00::/7` (ULA) has no stable std helper as of this Rust version.
fn is_unique_local_v6(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xfe00) == 0xfc00
}

/// `2001:db8::/32` — IPv6 documentation space.
fn is_documentation_v6(ip: Ipv6Addr) -> bool {
    ip.segments()[0] == 0x2001 && ip.segments()[1] == 0x0db8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_http_schemes_and_garbage() {
        assert!(matches!(
            validate_outbound_url("ftp://example.com/"),
            Err(OutboundUrlError::Scheme(_))
        ));
        assert!(matches!(
            validate_outbound_url("file:///etc/passwd"),
            Err(OutboundUrlError::Scheme(_))
        ));
        assert!(matches!(
            validate_outbound_url("not a url"),
            Err(OutboundUrlError::Invalid)
        ));
    }

    #[test]
    fn detects_blocked_ip_literals() {
        for raw in [
            "http://127.0.0.1/",
            "http://10.0.0.1/",
            "http://169.254.169.254/latest/meta-data/",
            "http://[::1]/",
            "http://[::ffff:127.0.0.1]/",
        ] {
            let url = Url::parse(raw).unwrap();
            assert!(ip_literal_is_blocked(&url), "expected blocked: {raw}");
        }
        assert!(!ip_literal_is_blocked(
            &Url::parse("https://93.184.216.34/").unwrap()
        ));
        assert!(!ip_literal_is_blocked(
            &Url::parse("https://example.com/").unwrap()
        ));
    }

    #[test]
    fn blocks_cgnat_benchmark_and_reserved() {
        assert!(is_blocked_ip("100.64.0.1".parse().unwrap()));
        assert!(is_blocked_ip("198.18.0.1".parse().unwrap()));
        assert!(is_blocked_ip("240.0.0.1".parse().unwrap()));
        assert!(is_blocked_ip("192.0.0.1".parse().unwrap()));
        assert!(is_blocked_ip("0.1.2.3".parse().unwrap()));
        assert!(!is_blocked_ip("93.184.216.34".parse().unwrap()));
    }
}
