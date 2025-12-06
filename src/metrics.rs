use prometheus_client::{encoding::text::encode, metrics::counter::Counter, registry::Registry};

#[derive(Debug)]
pub struct Metrics {
    registry: Registry,
    allowed: Counter,
    denied: Counter,
}

impl Metrics {
    pub fn new() -> Self {
        let mut registry = Registry::default();

        let allowed = Counter::default();
        let denied = Counter::default();

        registry.register(
            "rate_limiter_requests_allowed",
            "Total requests allowed by the rate limiter",
            allowed.clone(),
        );
        registry.register(
            "rate_limiter_requests_denied",
            "Total requests denied by the rate limiter",
            denied.clone(),
        );

        Self {
            registry,
            allowed,
            denied,
        }
    }

    pub fn record_allowed(&self) {
        self.allowed.inc();
    }

    pub fn record_denied(&self) {
        self.denied.inc();
    }

    pub fn export(&self) -> String {
        let mut buffer = String::new();
        encode(&mut buffer, &self.registry).unwrap_or_else(|_| {
            // Metrics encoding should never fail; expose empty metrics if it does.
            buffer.clear();
        });
        buffer
    }
}
