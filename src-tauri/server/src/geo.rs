//! Offline IP → country/city lookup backed by a MaxMind GeoLite2-City database.
//!
//! Privacy by design: no client IP ever leaves the server — geolocation is
//! resolved against a local `.mmdb` file. The whole feature is best-effort: if
//! the database is absent, the IP is private/unparseable, or the lookup misses,
//! we return `(None, None)` and the caller proceeds without geo. It must never
//! block authentication.

use std::net::IpAddr;
use std::sync::RwLock;
use std::time::SystemTime;

use maxminddb::{MaxMindDBError, Reader, geoip2};

/// The database in memory and the modification time of the file it came from.
/// The time lets `reload_if_changed` notice a monthly refresh without a restart.
struct Loaded {
    reader: Option<Reader<Vec<u8>>>,
    modified: Option<SystemTime>,
}

pub struct GeoIp {
    path: Option<String>,
    loaded: RwLock<Loaded>,
}

impl GeoIp {
    /// Load the database from `path`. A missing path or unreadable file disables
    /// geolocation (lookups return `(None, None)`) rather than failing startup.
    pub fn open(path: Option<&str>) -> Self {
        let loaded = match path {
            Some(p) => read_database(p),
            None => Loaded { reader: None, modified: None },
        };
        Self {
            path: path.map(str::to_string),
            loaded: RwLock::new(loaded),
        }
    }

    /// Re-read the file when it was replaced since the last load. The host
    /// refreshes the database once a month (`deploy/remote-deploy.sh`); without
    /// this the server kept answering from the file it opened at startup until
    /// the next restart. A file that fails to open keeps the database already
    /// in memory: a half-written download must not switch geolocation off.
    pub fn reload_if_changed(&self) {
        let Some(path) = &self.path else { return };
        let Ok(modified) = std::fs::metadata(path).and_then(|m| m.modified()) else {
            return;
        };
        let seen = self.loaded.read().map(|l| l.modified).unwrap_or(None);
        if seen == Some(modified) {
            return;
        }
        let fresh = read_database(path);
        if fresh.reader.is_none() {
            return;
        }
        if let Ok(mut slot) = self.loaded.write() {
            *slot = fresh;
        }
    }

    /// Whether a database is loaded; lookups return nothing without one.
    pub fn is_loaded(&self) -> bool {
        self.loaded.read().map(|l| l.reader.is_some()).unwrap_or(false)
    }

    /// Resolve `(country_code, city)` for a textual IP. Best-effort: any failure
    /// (no DB, unparseable IP, private range, lookup miss) yields `(None, None)`.
    pub fn lookup(&self, ip: &str) -> (Option<String>, Option<String>) {
        let Ok(addr) = ip.trim().parse::<IpAddr>() else {
            return (None, None);
        };
        // `::ffff:8.8.8.8` is an IPv4 address written the IPv6 way. The database
        // holds it under the IPv4 branch, so the mapped form found nothing.
        let addr = addr.to_canonical();
        if !is_public(&addr) {
            return (None, None);
        }
        let Ok(loaded) = self.loaded.read() else {
            return (None, None);
        };
        let Some(reader) = &loaded.reader else {
            return (None, None);
        };
        match reader.lookup::<geoip2::City>(addr) {
            Ok(rec) => {
                let country = rec
                    .country
                    .and_then(|c| c.iso_code)
                    .map(|c| c.to_ascii_uppercase());
                let city = rec
                    .city
                    .and_then(|c| c.names)
                    .and_then(|n| n.get("en").map(|s| s.to_string()));
                (country, city)
            }
            Err(MaxMindDBError::AddressNotFoundError(_)) => (None, None),
            Err(e) => {
                // A damaged file is not "no data for this address"; say so.
                tracing::warn!("GeoIP lookup failed: {e}");
                (None, None)
            }
        }
    }

    /// Country code only — what the language suggestion needs.
    pub fn country(&self, ip: &str) -> Option<String> {
        self.lookup(ip).0
    }
}

fn read_database(path: &str) -> Loaded {
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    match Reader::open_readfile(path) {
        Ok(reader) => {
            tracing::info!("GeoIP database loaded from {path}");
            Loaded { reader: Some(reader), modified }
        }
        Err(e) => {
            tracing::warn!("GeoIP database not loaded ({path}): {e} — geolocation disabled");
            Loaded { reader: None, modified }
        }
    }
}

/// The site language a country most likely reads, among the languages the site
/// has. `None` means "no better guess than the default": that is also the answer
/// for English-speaking countries and for countries whose language the site
/// does not have, where a wrong guess would be worse than English.
///
/// Switzerland gets German (the largest of its languages and one the site has);
/// a Swiss reader whose browser asks for Italian is already served by the
/// browser's own setting, which is checked first.
pub fn language_of_country(code: &str) -> Option<&'static str> {
    match code.to_ascii_uppercase().as_str() {
        "DE" | "AT" | "CH" | "LI" => Some("de"),
        "ES" | "MX" | "AR" | "CO" | "CL" | "PE" | "VE" | "EC" | "GT" | "CU" | "BO" | "DO"
        | "HN" | "PY" | "SV" | "NI" | "CR" | "PA" | "UY" | "PR" => Some("es"),
        "IT" | "SM" | "VA" => Some("it"),
        "UA" => Some("uk"),
        "RU" | "BY" | "KZ" | "KG" => Some("ru"),
        _ => None,
    }
}

/// Whether the address can belong to a place at all. Private, loopback,
/// link-local, shared (carrier-grade NAT), documentation, benchmarking,
/// multicast and reserved ranges cannot be geolocated and must not be stored as
/// if they were public. Expects an address already made canonical.
fn is_public(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || v4.is_documentation()
                || o[0] == 0 // 0.0.0.0/8, "this network"
                || (o[0] == 100 && (o[1] & 0xc0) == 64) // 100.64.0.0/10, carrier-grade NAT
                || (o[0] == 192 && o[1] == 0 && o[2] == 0) // 192.0.0.0/24, protocol assignments
                || (o[0] == 198 && (o[1] & 0xfe) == 18) // 198.18.0.0/15, benchmarking
                || o[0] >= 240) // 240.0.0.0/4, reserved
        }
        IpAddr::V6(v6) => {
            let s = v6.segments();
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (s[0] & 0xfe00) == 0xfc00 // fc00::/7, unique local
                || (s[0] & 0xffc0) == 0xfe80 // fe80::/10, link-local
                || (s[0] == 0x2001 && s[1] == 0x0db8)) // 2001:db8::/32, documentation
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn public(ip: &str) -> bool {
        is_public(&ip.parse::<IpAddr>().unwrap().to_canonical())
    }

    #[test]
    fn real_addresses_are_public() {
        for ip in ["8.8.8.8", "91.198.174.192", "2a00:1450:4001:81b::200e", "::ffff:8.8.8.8"] {
            assert!(public(ip), "{ip}");
        }
    }

    #[test]
    fn addresses_without_a_place_are_not_public() {
        for ip in [
            "10.1.2.3", "172.16.0.1", "192.168.1.1", "127.0.0.1", "169.254.1.1", "0.0.0.0",
            "0.1.2.3", "100.64.0.1", "100.127.255.255", "192.0.0.5", "198.18.0.1", "198.19.255.1",
            "203.0.113.9", "224.0.0.1", "255.255.255.255", "240.0.0.1", "::1", "::",
            "fc00::1", "fd12:3456::1", "fe80::1", "ff02::1", "2001:db8::1", "::ffff:10.0.0.1",
            "::ffff:127.0.0.1",
        ] {
            assert!(!public(ip), "{ip}");
        }
        // Neighbours of the shared-address and benchmarking blocks stay public.
        for ip in ["100.63.255.255", "100.128.0.1", "198.17.255.255", "198.20.0.1"] {
            assert!(public(ip), "{ip}");
        }
    }

    #[test]
    fn without_a_database_nothing_is_found() {
        let geo = GeoIp::open(None);
        assert_eq!(geo.lookup("8.8.8.8"), (None, None));
        assert!(!geo.is_loaded());
        geo.reload_if_changed(); // no path, nothing to do
    }

    #[test]
    fn a_missing_file_disables_geo_without_failing() {
        let geo = GeoIp::open(Some("/nonexistent/city.mmdb"));
        assert!(!geo.is_loaded());
        assert_eq!(geo.country("8.8.8.8"), None);
        geo.reload_if_changed();
        assert!(!geo.is_loaded());
    }

    #[test]
    fn country_decides_language_only_where_the_site_has_one() {
        assert_eq!(language_of_country("DE"), Some("de"));
        assert_eq!(language_of_country("at"), Some("de"));
        assert_eq!(language_of_country("MX"), Some("es"));
        assert_eq!(language_of_country("IT"), Some("it"));
        assert_eq!(language_of_country("UA"), Some("uk"));
        assert_eq!(language_of_country("RU"), Some("ru"));
        // English-speaking and unserved countries get no guess.
        for code in ["US", "GB", "CA", "AU", "FR", "PL", "BR", "JP", "", "ZZ"] {
            assert_eq!(language_of_country(code), None, "{code}");
        }
    }
}
