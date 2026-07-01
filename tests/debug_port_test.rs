use darp::config::{self, TokenCtx};
use std::collections::HashSet;

fn ctx() -> TokenCtx<'static> {
    TokenCtx {
        domain: "uhin",
        group: "laravel",
        service: "web-delivery-service",
        debug_port: 9004,
        proxy_port: Some(50103),
    }
}

// ---------------------------------------------------------------------------
// substitute_tokens
// ---------------------------------------------------------------------------

#[test]
fn substitutes_debug_port() {
    let out = config::substitute_tokens(
        "client_host=host.docker.internal client_port={debug_port}",
        &ctx(),
    );
    assert_eq!(out, "client_host=host.docker.internal client_port=9004");
}

#[test]
fn substitutes_all_known_tokens() {
    let out = config::substitute_tokens(
        "{service}.{group}.{domain}:{debug_port}/{proxy_port}",
        &ctx(),
    );
    assert_eq!(out, "web-delivery-service.laravel.uhin:9004/50103");
}

#[test]
fn substitutes_repeated_and_serve_command_style() {
    let out = config::substitute_tokens("dlv --listen=:{debug_port} --port {debug_port}", &ctx());
    assert_eq!(out, "dlv --listen=:9004 --port 9004");
}

#[test]
fn leaves_unknown_tokens_untouched() {
    let out = config::substitute_tokens("{pwd}/app:{debug_port}", &ctx());
    assert_eq!(out, "{pwd}/app:9004");
}

#[test]
fn proxy_port_token_untouched_when_absent() {
    let c = TokenCtx {
        proxy_port: None,
        ..ctx()
    };
    let out = config::substitute_tokens("port={proxy_port} dbg={debug_port}", &c);
    assert_eq!(out, "port={proxy_port} dbg=9004");
}

#[test]
fn no_tokens_is_identity() {
    assert_eq!(
        config::substitute_tokens("XDEBUG_MODE=debug", &ctx()),
        "XDEBUG_MODE=debug"
    );
}

// ---------------------------------------------------------------------------
// portmap_debug_port / portmap_proxy_port
// ---------------------------------------------------------------------------

#[test]
fn reads_debug_and_proxy_ports_from_portmap() {
    let pm = serde_json::json!({
        "uhin": {
            "laravel": {
                "web-delivery-service": { "port": 50103, "type": "http", "debug_port": 9004 }
            }
        }
    });
    assert_eq!(
        config::portmap_debug_port(&pm, "uhin", "laravel", "web-delivery-service"),
        Some(9004)
    );
    assert_eq!(
        config::portmap_proxy_port(&pm, "uhin", "laravel", "web-delivery-service"),
        Some(50103)
    );
}

#[test]
fn missing_debug_port_is_none() {
    // Stale portmap written before the debug_port feature.
    let pm = serde_json::json!({
        "uhin": { "laravel": { "svc": { "port": 50100, "type": "http" } } }
    });
    assert_eq!(
        config::portmap_debug_port(&pm, "uhin", "laravel", "svc"),
        None
    );
    assert_eq!(
        config::portmap_proxy_port(&pm, "uhin", "laravel", "svc"),
        Some(50100)
    );
}

#[test]
fn legacy_bare_number_proxy_port() {
    // Oldest portmap format: entry is a bare number, not an object.
    let pm = serde_json::json!({ "uhin": { ".": { "svc": 50100 } } });
    assert_eq!(
        config::portmap_proxy_port(&pm, "uhin", ".", "svc"),
        Some(50100)
    );
    assert_eq!(config::portmap_debug_port(&pm, "uhin", ".", "svc"), None);
}

#[test]
fn unknown_service_is_none() {
    let pm = serde_json::json!({});
    assert_eq!(
        config::portmap_debug_port(&pm, "uhin", "laravel", "nope"),
        None
    );
    assert_eq!(
        config::portmap_proxy_port(&pm, "uhin", "laravel", "nope"),
        None
    );
}

// ---------------------------------------------------------------------------
// Port-range hardening: base, skip-list, choose_debug_port
// ---------------------------------------------------------------------------

#[test]
fn debug_port_base_default_is_13000() {
    assert_eq!(config::DEBUG_PORT_BASE, 13000);
}

#[test]
fn well_known_ports_are_skipped() {
    let skip = config::well_known_skip_ports();
    assert!(skip.contains(&9000)); // php-fpm
    assert!(skip.contains(&9090)); // prometheus
    assert!(skip.contains(&9092)); // kafka
}

#[test]
fn reuses_in_range_persisted_port_without_advancing() {
    let skip = HashSet::new();
    let mut reserved: HashSet<u16> = [13005].into_iter().collect();
    let mut next = 13000;
    let p = config::choose_debug_port(Some(13005), 13000, &skip, &mut reserved, &mut next);
    assert_eq!(p, 13005);
    assert_eq!(next, 13000, "reuse must not advance the cursor");
}

#[test]
fn migrates_below_base_persisted_port_into_range() {
    // An old 9003-era assignment moves into the new range on re-deploy.
    let skip = HashSet::new();
    let mut reserved = HashSet::new();
    let mut next = 13000;
    let p = config::choose_debug_port(Some(9003), 13000, &skip, &mut reserved, &mut next);
    assert_eq!(p, 13000);
    assert!(reserved.contains(&13000));
    assert_eq!(next, 13001);
}

#[test]
fn reassigns_persisted_port_that_is_now_skipped() {
    let skip: HashSet<u16> = [13000].into_iter().collect();
    let mut reserved = HashSet::new();
    let mut next = 13000;
    let p = config::choose_debug_port(Some(13000), 13000, &skip, &mut reserved, &mut next);
    assert_eq!(
        p, 13001,
        "skipped persisted port must be reassigned past the skip-list"
    );
}

#[test]
fn new_assignment_skips_reserved_and_skiplist() {
    let skip: HashSet<u16> = [13001, 13002].into_iter().collect();
    let mut reserved: HashSet<u16> = [13000].into_iter().collect();
    let mut next = 13000;
    let p = config::choose_debug_port(None, 13000, &skip, &mut reserved, &mut next);
    assert_eq!(p, 13003); // 13000 reserved, 13001/13002 skipped
}

#[test]
fn keeps_persisted_and_new_service_gets_distinct_port() {
    // Mirrors deploy.rs: reserved is seeded with valid persisted ports up front, so a
    // kept port is never handed to another service processed later.
    let skip = HashSet::new();
    let mut reserved: HashSet<u16> = [13000].into_iter().collect();
    let mut next = 13000;
    let a = config::choose_debug_port(Some(13000), 13000, &skip, &mut reserved, &mut next);
    let b = config::choose_debug_port(None, 13000, &skip, &mut reserved, &mut next);
    assert_eq!(a, 13000);
    assert_eq!(b, 13001);
    assert_ne!(a, b);
}

#[test]
fn cursor_below_base_jumps_to_base() {
    let skip = HashSet::new();
    let mut reserved = HashSet::new();
    let mut next = 0;
    let p = config::choose_debug_port(None, 13000, &skip, &mut reserved, &mut next);
    assert_eq!(p, 13000);
}
