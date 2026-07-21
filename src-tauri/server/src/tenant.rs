//! Multi-tenancy foundation.
//!
//! A *tenant* is one artist/maker site. Each tenant's content lives in its own
//! Postgres schema (`schema_name`). Tenant #1 (Ritunia, the flagship) maps to the
//! pre-existing `public` schema, so introducing multi-tenancy moves no data.
//!
//! This module is additive: it loads the tenant table and resolves the tenant for a
//! request from the Host header. Making the query/service layer actually honor the
//! resolved schema (`SET search_path`) is the next increment — see PLATFORM_ROADMAP.md.

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};

use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use uuid::Uuid;

/// One artist/maker site.
#[derive(Debug, Clone)]
pub struct Tenant {
    pub id: Uuid,
    /// Subdomain label, e.g. `ritunia` in `ritunia.example.com`.
    pub slug: String,
    /// Postgres schema holding this tenant's content. `public` for the flagship.
    pub schema_name: String,
    /// Optional apex/custom domain that also maps to this tenant.
    pub custom_domain: Option<String>,
    /// `active` | `suspended`.
    pub status: String,
    /// The flagship tenant, used as the resolver fallback so unknown hosts (apex,
    /// `www`, localhost in dev) still land on a valid site.
    pub is_primary: bool,
}

/// In-memory snapshot of all tenants, loaded once at startup and resolved per request.
///
/// A later increment will make this refreshable when a tenant is provisioned; today it
/// is immutable after load, which is fine while provisioning is not yet implemented.
pub struct TenantRegistry {
    by_subdomain: HashMap<String, Arc<Tenant>>,
    by_domain: HashMap<String, Arc<Tenant>>,
    primary: Arc<Tenant>,
}

impl TenantRegistry {
    /// Load every tenant from `platform.tenants`. Fails if the table is empty: the
    /// migration seeds the primary, so an empty table means the migration did not run —
    /// better to fail loudly than to serve every request with no tenant.
    pub async fn load(pool: &PgPool) -> anyhow::Result<Arc<Self>> {
        let rows = sqlx::query_as::<_, (Uuid, String, String, Option<String>, String, bool)>(
            "SELECT id, slug, schema_name, custom_domain, status, is_primary \
             FROM platform.tenants ORDER BY created_at",
        )
        .fetch_all(pool)
        .await?;

        let mut by_subdomain: HashMap<String, Arc<Tenant>> = HashMap::new();
        let mut by_domain: HashMap<String, Arc<Tenant>> = HashMap::new();
        let mut primary: Option<Arc<Tenant>> = None;
        let mut first: Option<Arc<Tenant>> = None;

        for (id, slug, schema_name, custom_domain, status, is_primary) in rows {
            let tenant = Arc::new(Tenant {
                id,
                slug,
                schema_name,
                custom_domain,
                status,
                is_primary,
            });
            tracing::debug!(
                target: "gotiga_server::tenant",
                id = %tenant.id,
                slug = %tenant.slug,
                schema = %tenant.schema_name,
                domain = ?tenant.custom_domain,
                status = %tenant.status,
                is_primary = tenant.is_primary,
                "tenant loaded"
            );
            if tenant.is_primary {
                primary = Some(tenant.clone());
            }
            if first.is_none() {
                first = Some(tenant.clone());
            }
            by_subdomain.insert(tenant.slug.clone(), tenant.clone());
            if let Some(domain) = &tenant.custom_domain {
                by_domain.insert(domain.to_ascii_lowercase(), tenant.clone());
            }
        }

        let primary = primary.or(first).ok_or_else(|| {
            anyhow::anyhow!("platform.tenants is empty — expected at least the seeded primary tenant")
        })?;

        tracing::info!(
            target: "gotiga_server::tenant",
            count = by_subdomain.len(),
            primary = %primary.slug,
            "tenant registry loaded"
        );

        Ok(Arc::new(Self {
            by_subdomain,
            by_domain,
            primary,
        }))
    }

    /// Resolve the tenant for a request `Host`. Match order: exact custom domain,
    /// then the first subdomain label, then the primary tenant as fallback (so apex,
    /// `www`, and dev hosts like localhost still resolve to a valid site).
    pub fn resolve(&self, host: &str) -> Arc<Tenant> {
        let host = host
            .split(':')
            .next()
            .unwrap_or(host)
            .trim()
            .to_ascii_lowercase();

        if host.is_empty() {
            return self.primary.clone();
        }
        if let Some(tenant) = self.by_domain.get(&host) {
            return tenant.clone();
        }
        if let Some(label) = host.split('.').next() {
            if let Some(tenant) = self.by_subdomain.get(label) {
                return tenant.clone();
            }
        }
        self.primary.clone()
    }
}

/// Lazily-built, cached connection pools — one per tenant schema. Each pool pins its
/// connections to the tenant's schema via `SET search_path` on connect, so the existing
/// (unqualified) queries in the `Repository` land in the right schema with **no per-query
/// changes**. Pools are created lazily (`connect_lazy_with`) and kept small.
#[derive(Clone)]
pub struct TenantPools {
    base: PgConnectOptions,
    pools: Arc<StdMutex<HashMap<String, PgPool>>>,
}

impl TenantPools {
    /// Parse the base connection options from the app's `DATABASE_URL` once.
    pub fn new(database_url: &str) -> anyhow::Result<Self> {
        let base: PgConnectOptions = database_url.parse()?;
        Ok(Self {
            base,
            pools: Arc::new(StdMutex::new(HashMap::new())),
        })
    }

    /// Get (or lazily create) the pool bound to `schema`. `schema` is a validated
    /// identifier (see the CHECK on `platform.tenants`); it is still quoted defensively.
    pub fn pool_for(&self, schema: &str) -> PgPool {
        let mut pools = self.pools.lock().expect("tenant pool registry poisoned");
        if let Some(pool) = pools.get(schema) {
            return pool.clone();
        }
        let schema_owned = schema.to_string();
        let pool = PgPoolOptions::new()
            .max_connections(3)
            .after_connect(move |conn, _meta| {
                let schema = schema_owned.clone();
                Box::pin(async move {
                    let sql = format!("SET search_path TO \"{}\", public", schema);
                    sqlx::query(&sql).execute(conn).await?;
                    Ok(())
                })
            })
            .connect_lazy_with(self.base.clone());
        pools.insert(schema.to_string(), pool.clone());
        pool
    }
}
