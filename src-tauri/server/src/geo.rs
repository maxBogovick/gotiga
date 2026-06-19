//! Offline IP → country/city lookup backed by a MaxMind GeoLite2-City database.
//!
//! Privacy by design: no client IP ever leaves the server — geolocation is
//! resolved against a local `.mmdb` file. The whole feature is best-effort: if
//! the database is absent, the IP is private/unparseable, or the lookup misses,
//! we return `(None, None)` and the caller proceeds without geo. It must never
//! block authentication.

use std::net::IpAddr;

use maxminddb::{Reader, geoip2};

pub struct GeoIp {
    reader: Option<Reader<Vec<u8>>>,
}

impl GeoIp {
    /// Load the database from `path`. A missing path or unreadable file disables
    /// geolocation (lookups return `(None, None)`) rather than failing startup.
    pub fn open(path: Option<&str>) -> Self {
        let reader = path.and_then(|p| match Reader::open_readfile(p) {
            Ok(r) => {
                tracing::info!("GeoIP database loaded from {p}");
                Some(r)
            }
            Err(e) => {
                tracing::warn!("GeoIP database not loaded ({p}): {e} — geolocation disabled");
                None
            }
        });
        Self { reader }
    }

    /// Resolve `(country_code, city)` for a textual IP. Best-effort: any failure
    /// (no DB, unparseable IP, private range, lookup miss) yields `(None, None)`.
    pub fn lookup(&self, ip: &str) -> (Option<String>, Option<String>) {
        let Some(reader) = &self.reader else {
            return (None, None);
        };
        let Ok(addr) = ip.parse::<IpAddr>() else {
            return (None, None);
        };
        if is_private(&addr) {
            return (None, None);
        }
        match reader.lookup::<geoip2::City>(addr) {
            Ok(rec) => {
                let country = rec.country.and_then(|c| c.iso_code).map(str::to_string);
                let city = rec
                    .city
                    .and_then(|c| c.names)
                    .and_then(|n| n.get("en").map(|s| s.to_string()));
                (country, city)
            }
            Err(_) => (None, None),
        }
    }
}

/// Private, loopback, link-local and unspecified addresses can't be geolocated
/// and shouldn't be stored as if they were public — skip them.
fn is_private(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private() || v4.is_loopback() || v4.is_link_local() || v4.is_unspecified()
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified(),
    }
}
