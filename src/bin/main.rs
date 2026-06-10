use gss::load_gss_from_file;

fn main() {
    let s = load_gss_from_file("temp/temp.gss");
    assert!(s.is_ok(), "{s:?}");
    s.unwrap().dump(0);
}
