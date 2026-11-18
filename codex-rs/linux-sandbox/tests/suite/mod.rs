// Aggregates all former standalone integration tests as modules.
mod bundled_bwrap;
mod bwrap_path;
mod managed_proxy;
mod sandbox;

#[path = "lifetime_tests.rs"]
mod lifetime;
