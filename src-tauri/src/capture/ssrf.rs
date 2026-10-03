//! SSRF guard for every fetch the capture pipeline makes (the page itself,
//! and every localized content image) — vendored from the wraith
//! workspace's `wraith-assets` crate. Matters because asset URLs come from
//! attacker-influenced page content (a captured page can embed
//! `<img src="http://169.254.169.254/...">`).
//!
//! Two mechanisms, both necessary — confirmed empirically, not assumed: a
//! request whose URL already names a literal IP (`http://127.0.0.1/`)
//! never invokes `reqwest`'s configured [`Resolve`] at all (its connector
//! only resolves *hostnames*), so blocking the *resolver* alone leaves
//! literal-IP SSRF completely open. [`literal_ip_is_blocked`] is the
//! pre-check callers must run against a request's URL before ever handing
//! it to the client — see `capture::localize::fetch_one`.
//!
//! For hostnames, the custom [`Resolve`] impl is what actually closes the
//! gap: resolving here (rather than pre-checking a hostname's IPs and then
//! letting `reqwest` re-resolve independently) avoids a DNS rebinding
//! TOCTOU — the IP that gets connected to is exactly the IP this code just
//! checked, not a second, independent lookup that could return something
//! different.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use url::Url;

/// True if `ip` is loopback, link-local, private-use, unspecified,
/// broadcast, documentation-only, or carrier-grade-NAT (100.64.0.0/10) —
/// i.e. not a routable public address a legitimate asset should live at.
/// IPv4-mapped IPv6 addresses (`::ffff:127.0.0.1`) are unwrapped first, so
/// that representation can't slip past the IPv4 checks.
pub fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_v4(v4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(mapped) => is_blocked_v4(mapped),
            None => {
                v6.is_loopback()
                    || v6.is_unspecified()
                    || v6.is_unique_local()
                    || v6.is_unicast_link_local()
            }
        },
    }
}

fn is_blocked_v4(v4: Ipv4Addr) -> bool {
    v4.is_loopback()
        || v4.is_private()
        || v4.is_link_local()
        || v4.is_unspecified()
        || v4.is_broadcast()
        || v4.is_documentation()
        || is_carrier_grade_nat(v4)
}

/// 100.64.0.0/10 (RFC 6598) — not covered by [`Ipv4Addr::is_private`],
/// which only covers RFC 1918. `Ipv4Addr::is_shared` covers exactly this
/// but is still nightly-only (`#![feature(ip)]`), so it's checked directly
/// against the raw octets here instead.
fn is_carrier_grade_nat(v4: Ipv4Addr) -> bool {
    let [a, b, ..] = v4.octets();
    a == 100 && (64..=127).contains(&b)
}

/// If `url`'s host is already a literal IP address, returns whether that
/// address is blocked. Returns `false` (not blocked) for a hostname —
/// hostnames are guarded by [`GuardedResolver`] instead, at actual
/// resolution time.
pub fn literal_ip_is_blocked(url: &Url) -> bool {
    match url.host() {
        Some(url::Host::Ipv4(v4)) => is_blocked_v4(v4),
        Some(url::Host::Ipv6(v6)) => is_blocked_ip(IpAddr::V6(v6)),
        _ => false,
    }
}

/// A [`Resolve`] implementation that resolves a hostname exactly the way
/// the default resolver would (via [`tokio::net::lookup_host`]), then
/// drops any resolved address [`is_blocked_ip`] flags — so `reqwest` can
/// only ever connect to what's left. If every candidate address is
/// blocked, resolution fails outright rather than silently returning an
/// empty (and thus presumably differently-erroring) address list.
#[derive(Debug, Default)]
struct GuardedResolver;

impl Resolve for GuardedResolver {
    fn resolve(&self, name: Name) -> Resolving {
        Box::pin(async move {
            let host = name.as_str().to_string();
            let addrs = tokio::net::lookup_host((host.as_str(), 0))
                .await
                .map_err(|source| Box::new(source) as Box<dyn std::error::Error + Send + Sync>)?;
            let allowed: Vec<SocketAddr> = addrs.filter(|addr| !is_blocked_ip(addr.ip())).collect();
            if allowed.is_empty() {
                return Err(Box::new(SsrfBlocked(host)) as Box<dyn std::error::Error + Send + Sync>);
            }
            Ok(Box::new(allowed.into_iter()) as Addrs)
        })
    }
}

#[derive(Debug)]
struct SsrfBlocked(String);

impl std::fmt::Display for SsrfBlocked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "refusing to fetch {:?}: every resolved address is loopback/link-local/private",
            self.0
        )
    }
}

impl std::error::Error for SsrfBlocked {}

/// A [`reqwest::ClientBuilder`] with [`GuardedResolver`] installed —
/// callers still need [`literal_ip_is_blocked`] for the literal-IP case
/// this alone doesn't cover on the *first* request. The single entry point
/// every HTTP client the capture pipeline builds for fetching page/asset
/// content should start from.
///
/// Also installs a redirect policy that re-runs [`literal_ip_is_blocked`]
/// on every hop: a hostname redirect target is still safe (it goes through
/// [`GuardedResolver`] like any other request this client makes), but a
/// redirect straight to a literal IP — `Location: http://169.254.169.254/`
/// — would otherwise reach the connector exactly the way a literal-IP
/// *initial* URL does, bypassing the resolver entirely. A malicious or
/// compromised origin can fully control its own `Location` header, so this
/// has to be checked per hop, not just once up front.
pub fn ssrf_guarded_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .dns_resolver(std::sync::Arc::new(GuardedResolver))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if literal_ip_is_blocked(attempt.url()) {
                let blocked_url = attempt.url().to_string();
                return attempt.error(SsrfBlocked(blocked_url));
            }
            // Defers to the default policy for everything else, including
            // its 10-hop loop/length cap — this closure only adds the
            // per-hop literal-IP check on top.
            reqwest::redirect::Policy::default().redirect(attempt)
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_loopback_v4() {
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
    }

    #[test]
    fn blocks_rfc1918_private_ranges() {
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
    }

    #[test]
    fn blocks_link_local_and_metadata_endpoint() {
        // 169.254.169.254 — the AWS/GCP/Azure instance-metadata address,
        // the canonical real-world SSRF target this guard exists for.
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))));
    }

    #[test]
    fn blocks_carrier_grade_nat_range() {
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(100, 127, 255, 255))));
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(100, 63, 0, 1))));
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(100, 128, 0, 1))));
    }

    #[test]
    fn allows_ordinary_public_v4() {
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))));
    }

    #[test]
    fn blocks_loopback_and_unique_local_v6() {
        assert!(is_blocked_ip(IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)));
        assert!(is_blocked_ip(IpAddr::V6(std::net::Ipv6Addr::new(
            0xfd00, 0, 0, 0, 0, 0, 0, 1
        ))));
    }

    #[test]
    fn unwraps_ipv4_mapped_ipv6_before_checking() {
        let mapped = std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0x7f00, 0x0001);
        assert!(is_blocked_ip(IpAddr::V6(mapped)));
    }

    /// End-to-end: a real HTTP server standing in for a malicious/
    /// compromised origin that 302s to a literal loopback IP — the exact
    /// gap a per-hop check (as opposed to only checking the client's
    /// initial request URL) exists to close.
    #[tokio::test]
    async fn redirect_policy_blocks_literal_ip_hop() {
        use axum::Router;
        use axum::response::Redirect;
        use axum::routing::get;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        let app = Router::new().route(
            "/start",
            get(|| async { Redirect::temporary("http://127.0.0.1:9/evil") }),
        );
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });

        let client = ssrf_guarded_client_builder()
            .build()
            .expect("client should build");
        let result = client.get(format!("http://{addr}/start")).send().await;

        let err = result.expect_err("redirect to a literal loopback IP must be refused");
        use std::error::Error as _;
        assert!(
            err.to_string().contains("refusing to fetch")
                || err
                    .source()
                    .is_some_and(|s| s.to_string().contains("refusing to fetch")),
            "expected an SsrfBlocked error, got: {err}"
        );
    }

    #[test]
    fn literal_ip_precheck_catches_what_the_resolver_never_sees() {
        assert!(literal_ip_is_blocked(
            &Url::parse("http://127.0.0.1:9999/x").unwrap()
        ));
        assert!(literal_ip_is_blocked(
            &Url::parse("http://169.254.169.254/latest/meta-data/").unwrap()
        ));
        assert!(!literal_ip_is_blocked(
            &Url::parse("http://93.184.216.34/x").unwrap()
        ));
        assert!(!literal_ip_is_blocked(
            &Url::parse("http://example.com/x").unwrap()
        ));
    }
}
