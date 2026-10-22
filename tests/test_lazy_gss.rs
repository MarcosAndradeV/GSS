use gss::lazy_gss;

lazy_gss! {
    pub static CONFIG = "./tests/test_config.gss";

    pub get_port() -> String = ["net", "port"] or_else "8080".to_string();
    pub get_host() -> String = ["net", "host"] or_else "127.0.0.1".to_string();
    pub get_workers() -> usize = ["system", "workers"] or_else 4;
}

#[test]
fn test_getters() {
    let port = get_port();
    assert_eq!(port.as_str(), "8070");
    let host = get_host();
    assert_eq!(host.as_str(), "127.0.0.1");
    let workers = get_workers();
    assert_eq!(workers, 8);
}
