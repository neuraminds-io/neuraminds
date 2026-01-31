use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::header,
    HttpResponse,
};
use futures::future::{ok, LocalBoxFuture, Ready};
use log::{info, warn};
use std::collections::HashSet;
use std::sync::OnceLock;

/// Blocked countries by ISO 3166-1 alpha-2 code
static BLOCKED_COUNTRIES: OnceLock<HashSet<&'static str>> = OnceLock::new();

fn get_blocked_countries() -> &'static HashSet<&'static str> {
    BLOCKED_COUNTRIES.get_or_init(|| {
        let mut set = HashSet::new();
        // US and territories
        set.insert("US");
        set.insert("UM"); // US Minor Outlying Islands
        set.insert("PR"); // Puerto Rico
        set.insert("VI"); // US Virgin Islands
        set.insert("GU"); // Guam
        set.insert("AS"); // American Samoa

        // UK
        set.insert("GB");

        // Sanctioned countries
        set.insert("CU"); // Cuba
        set.insert("IR"); // Iran
        set.insert("KP"); // North Korea
        set.insert("SY"); // Syria
        set.insert("RU"); // Russia

        // Other restricted jurisdictions
        set.insert("AU"); // Australia (depending on license)
        set.insert("CA"); // Canada (Ontario specifically, but blocking all for safety)
        set
    })
}

/// Geo-blocking middleware
///
/// Blocks requests from prohibited jurisdictions based on:
/// 1. CF-IPCountry header (Cloudflare)
/// 2. X-Vercel-IP-Country header (Vercel)
/// 3. GeoIP headers from other CDNs
pub struct GeoBlock {
    enabled: bool,
}

impl GeoBlock {
    pub fn new(enabled: bool) -> Self {
        if enabled {
            info!("Geo-blocking enabled for {} countries", get_blocked_countries().len());
        } else {
            warn!("Geo-blocking DISABLED - not recommended for production");
        }
        Self { enabled }
    }
}

impl<S, B> Transform<S, ServiceRequest> for GeoBlock
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = actix_web::Error;
    type Transform = GeoBlockMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(GeoBlockMiddleware {
            service,
            enabled: self.enabled,
        })
    }
}

pub struct GeoBlockMiddleware<S> {
    service: S,
    enabled: bool,
}

impl<S, B> Service<ServiceRequest> for GeoBlockMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Skip geo-blocking if disabled
        if !self.enabled {
            let fut = self.service.call(req);
            return Box::pin(async move {
                let res = fut.await?;
                Ok(res.map_into_left_body())
            });
        }

        // Skip geo-blocking for health checks
        let path = req.path();
        if path.starts_with("/health") || path == "/metrics" || path == "/metrics/prometheus" {
            let fut = self.service.call(req);
            return Box::pin(async move {
                let res = fut.await?;
                Ok(res.map_into_left_body())
            });
        }

        // Check country from various CDN headers
        let country = get_country_from_headers(&req);

        if let Some(country_code) = &country {
            if get_blocked_countries().contains(country_code.as_str()) {
                warn!(
                    "Blocked request from {} (IP: {:?})",
                    country_code,
                    req.connection_info().realip_remote_addr()
                );

                let response = HttpResponse::Forbidden()
                    .insert_header((header::CONTENT_TYPE, "application/json"))
                    .body(r#"{"error":{"code":"REGION_BLOCKED","message":"This service is not available in your region"}}"#);

                return Box::pin(async move {
                    Ok(req.into_response(response).map_into_right_body())
                });
            }
        }

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res.map_into_left_body())
        })
    }
}

/// Extract country code from CDN headers
fn get_country_from_headers(req: &ServiceRequest) -> Option<String> {
    let headers = req.headers();

    // Try Cloudflare header first
    if let Some(cf) = headers.get("CF-IPCountry") {
        if let Ok(country) = cf.to_str() {
            if country != "XX" && country != "T1" {
                // XX = unknown, T1 = Tor
                return Some(country.to_uppercase());
            }
        }
    }

    // Try Vercel header
    if let Some(vercel) = headers.get("X-Vercel-IP-Country") {
        if let Ok(country) = vercel.to_str() {
            return Some(country.to_uppercase());
        }
    }

    // Try AWS CloudFront header
    if let Some(cf) = headers.get("CloudFront-Viewer-Country") {
        if let Ok(country) = cf.to_str() {
            return Some(country.to_uppercase());
        }
    }

    // Try generic X-Country header (some load balancers use this)
    if let Some(generic) = headers.get("X-Country") {
        if let Ok(country) = generic.to_str() {
            return Some(country.to_uppercase());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocked_countries_initialized() {
        let blocked = get_blocked_countries();
        assert!(blocked.contains("US"));
        assert!(blocked.contains("GB"));
        assert!(blocked.contains("KP"));
        assert!(!blocked.contains("DE"));
        assert!(!blocked.contains("JP"));
    }

    #[test]
    fn test_country_list_completeness() {
        let blocked = get_blocked_countries();
        // Ensure minimum set of critical blocked countries
        assert!(blocked.len() >= 10);
        assert!(blocked.contains("US"));
        assert!(blocked.contains("IR"));
        assert!(blocked.contains("CU"));
    }
}
