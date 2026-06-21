use axum::{
    extract::{MatchedPath, Request, State},
    http::{HeaderName, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use uuid::Uuid;

pub const REQUEST_ID_HEADER: &str = "x-request-id";

const LATENCY_BUCKETS_SECONDS: [f64; 10] = [
    0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.5, 5.0,
];
const MAX_HTTP_SERIES: usize = 1024;
const OVERFLOW_ROUTE: &str = "/__overflow__";

#[derive(Clone, Default)]
pub struct ObservabilityState {
    inner: Arc<Mutex<MetricsState>>,
}

#[derive(Default)]
struct MetricsState {
    http: HashMap<HttpKey, HttpMetrics>,
    business: HashMap<BusinessKey, u64>,
    in_flight: u64,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct HttpKey {
    method: String,
    route: String,
    status_class: String,
}

#[derive(Clone, Default)]
struct HttpMetrics {
    total: u64,
    duration_sum_seconds: f64,
    buckets: [u64; LATENCY_BUCKETS_SECONDS.len()],
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct BusinessKey {
    event: &'static str,
    result: &'static str,
}

#[derive(Clone, Default)]
struct MetricsSnapshot {
    http: Vec<(HttpKey, HttpMetrics)>,
    business: Vec<(BusinessKey, u64)>,
    in_flight: u64,
}

impl ObservabilityState {
    fn request_started(&self) {
        let mut state = self.inner.lock().expect("observability mutex poisoned");
        state.in_flight = state.in_flight.saturating_add(1);
    }

    fn request_finished(
        &self,
        method: &str,
        route: &str,
        status: StatusCode,
        elapsed_seconds: f64,
    ) {
        let status_class = format!("{}xx", status.as_u16() / 100);
        let mut key = HttpKey {
            method: method.to_string(),
            route: route.to_string(),
            status_class,
        };
        let mut state = self.inner.lock().expect("observability mutex poisoned");
        state.in_flight = state.in_flight.saturating_sub(1);
        if !state.http.contains_key(&key) && state.http.len() >= MAX_HTTP_SERIES {
            key.route = OVERFLOW_ROUTE.to_string();
        }
        let metrics = state.http.entry(key).or_default();
        metrics.total += 1;
        metrics.duration_sum_seconds += elapsed_seconds;
        for (idx, bucket) in LATENCY_BUCKETS_SECONDS.iter().enumerate() {
            if elapsed_seconds <= *bucket {
                metrics.buckets[idx] += 1;
            }
        }
    }

    pub fn record_business_event(&self, event: &'static str, result: &'static str) {
        let mut state = self.inner.lock().expect("observability mutex poisoned");
        *state
            .business
            .entry(BusinessKey { event, result })
            .or_insert(0) += 1;
    }

    pub fn render_prometheus(&self) -> String {
        let state = self.snapshot();
        let mut out = String::new();

        out.push_str("# HELP gotiga_build_info Static service build metadata.\n");
        out.push_str("# TYPE gotiga_build_info gauge\n");
        out.push_str(&format!(
            "gotiga_build_info{{version=\"{}\"}} 1\n",
            env!("CARGO_PKG_VERSION")
        ));

        out.push_str("# HELP gotiga_http_requests_in_flight Currently executing HTTP requests.\n");
        out.push_str("# TYPE gotiga_http_requests_in_flight gauge\n");
        out.push_str(&format!(
            "gotiga_http_requests_in_flight {}\n",
            state.in_flight
        ));

        out.push_str("# HELP gotiga_http_requests_total Total HTTP requests by method, route and status class.\n");
        out.push_str("# TYPE gotiga_http_requests_total counter\n");
        for (key, value) in &state.http {
            out.push_str(&format!(
                "gotiga_http_requests_total{{method=\"{}\",route=\"{}\",status_class=\"{}\"}} {}\n",
                esc(&key.method),
                esc(&key.route),
                esc(&key.status_class),
                value.total
            ));
        }

        out.push_str(
            "# HELP gotiga_http_request_duration_seconds HTTP request latency histogram.\n",
        );
        out.push_str("# TYPE gotiga_http_request_duration_seconds histogram\n");
        for (key, value) in &state.http {
            for (idx, bucket) in LATENCY_BUCKETS_SECONDS.iter().enumerate() {
                out.push_str(&format!(
                    "gotiga_http_request_duration_seconds_bucket{{method=\"{}\",route=\"{}\",status_class=\"{}\",le=\"{}\"}} {}\n",
                    esc(&key.method),
                    esc(&key.route),
                    esc(&key.status_class),
                    format_bucket(*bucket),
                    value.buckets[idx]
                ));
            }
            out.push_str(&format!(
                "gotiga_http_request_duration_seconds_bucket{{method=\"{}\",route=\"{}\",status_class=\"{}\",le=\"+Inf\"}} {}\n",
                esc(&key.method),
                esc(&key.route),
                esc(&key.status_class),
                value.total
            ));
            out.push_str(&format!(
                "gotiga_http_request_duration_seconds_sum{{method=\"{}\",route=\"{}\",status_class=\"{}\"}} {:.6}\n",
                esc(&key.method),
                esc(&key.route),
                esc(&key.status_class),
                value.duration_sum_seconds
            ));
            out.push_str(&format!(
                "gotiga_http_request_duration_seconds_count{{method=\"{}\",route=\"{}\",status_class=\"{}\"}} {}\n",
                esc(&key.method),
                esc(&key.route),
                esc(&key.status_class),
                value.total
            ));
        }

        out.push_str(
            "# HELP gotiga_business_events_total Business-domain events with bounded labels.\n",
        );
        out.push_str("# TYPE gotiga_business_events_total counter\n");
        for (key, value) in &state.business {
            out.push_str(&format!(
                "gotiga_business_events_total{{event=\"{}\",result=\"{}\"}} {}\n",
                esc(key.event),
                esc(key.result),
                value
            ));
        }

        out
    }

    fn snapshot(&self) -> MetricsSnapshot {
        let state = self.inner.lock().expect("observability mutex poisoned");
        let mut http = state
            .http
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>();
        http.sort_by(|a, b| a.0.cmp(&b.0));

        let mut business = state
            .business
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect::<Vec<_>>();
        business.sort_by(|a, b| a.0.cmp(&b.0));

        MetricsSnapshot {
            http,
            business,
            in_flight: state.in_flight,
        }
    }
}

pub async fn request_observability_middleware(
    State(observability): State<ObservabilityState>,
    mut request: Request,
    next: Next,
) -> Response {
    let request_id = request_id(&request);
    let request_id_value =
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("invalid"));
    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    let method = request.method().to_string();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| normalize_route(request.uri().path()));

    observability.request_started();
    let started = Instant::now();
    let mut response = next.run(request).await;
    let status = response.status();
    let elapsed = started.elapsed();
    observability.request_finished(&method, &route, status, elapsed.as_secs_f64());

    response
        .headers_mut()
        .insert(HeaderName::from_static(REQUEST_ID_HEADER), request_id_value);

    tracing::info!(
        target: "gotiga_server::http",
        request_id = %request_id,
        method = %method,
        route = %route,
        status = status.as_u16(),
        latency_ms = elapsed.as_millis(),
        "request completed"
    );

    response
}

#[derive(Clone, Debug)]
pub struct RequestId(pub String);

fn request_id(request: &Request) -> String {
    request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|v| valid_request_id(v))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

fn valid_request_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
}

fn normalize_route(path: &str) -> String {
    let segments = path.split('/').collect::<Vec<_>>();
    let normalized = segments
        .iter()
        .enumerate()
        .map(|(idx, segment)| {
            if segment.is_empty() {
                ""
            } else if looks_like_dynamic_segment(segment)
                || previous_segment_marks_token(&segments, idx)
            {
                ":id"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/");
    if normalized.is_empty() {
        "/".to_string()
    } else {
        normalized
    }
}

fn looks_like_dynamic_segment(segment: &str) -> bool {
    let hyphens = segment.as_bytes().iter().filter(|b| **b == b'-').count();
    let mostly_hex_or_hyphen = segment.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-');
    (segment.len() == 36 && hyphens == 4 && mostly_hex_or_hyphen)
        || (segment.len() == 19 && hyphens == 3 && mostly_hex_or_hyphen)
        || segment.len() > 48
        || segment.starts_with("tok_")
        || segment.starts_with("cert_")
}

fn previous_segment_marks_token(segments: &[&str], idx: usize) -> bool {
    let Some(prev) = idx.checked_sub(1).and_then(|i| segments.get(i)).copied() else {
        return false;
    };
    matches!(
        prev,
        "cancel" | "leave" | "notify" | "certificates" | "commissions" | "reset-token"
    )
}

fn format_bucket(bucket: f64) -> String {
    if bucket.fract() == 0.0 {
        format!("{bucket:.0}")
    } else {
        format!("{bucket:.3}")
    }
}

fn esc(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('"', "\\\"")
}

pub async fn metrics_handler(State(observability): State<ObservabilityState>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        observability.render_prometheus(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prometheus_render_uses_bounded_business_labels() {
        let state = ObservabilityState::default();
        state.record_business_event("booking_created", "ok");
        let rendered = state.render_prometheus();
        assert!(rendered.contains("gotiga_business_events_total"));
        assert!(rendered.contains("event=\"booking_created\""));
        assert!(rendered.contains("result=\"ok\""));
    }

    #[test]
    fn rejects_request_ids_that_can_break_logs_or_headers() {
        assert!(valid_request_id("abc-123_ok"));
        assert!(!valid_request_id(""));
        assert!(!valid_request_id("bad\nid"));
        assert!(!valid_request_id(&"x".repeat(129)));
    }

    #[test]
    fn normalizes_dynamic_fallback_paths_for_metric_labels() {
        assert_eq!(
            normalize_route("/api/v1/figurines/550e8400-e29b-41d4-a716-446655440000"),
            "/api/v1/figurines/:id"
        );
        let long_token_path = format!("/api/v1/bookings/cancel/{}", "x".repeat(64));
        assert_eq!(
            normalize_route(&long_token_path),
            "/api/v1/bookings/cancel/:id"
        );
        assert_eq!(
            normalize_route("/api/v1/bookings/cancel/ABCD-EF12-3456-7890"),
            "/api/v1/bookings/cancel/:id"
        );
        assert_eq!(
            normalize_route("/api/v1/commissions/short-secret"),
            "/api/v1/commissions/:id"
        );
    }

    #[test]
    fn caps_http_metric_series_to_bound_memory() {
        let state = ObservabilityState::default();
        for i in 0..(MAX_HTTP_SERIES + 3) {
            state.request_finished("GET", &format!("/r/{i}"), StatusCode::OK, 0.001);
        }
        let snapshot = state.snapshot();
        assert_eq!(snapshot.http.len(), MAX_HTTP_SERIES + 1);
        assert!(
            snapshot
                .http
                .iter()
                .any(|(key, _)| key.route == OVERFLOW_ROUTE)
        );
    }
}
