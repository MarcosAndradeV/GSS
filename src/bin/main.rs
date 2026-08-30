use gss::load_gss_from_file;

fn main() {
    let s = load_gss_from_file("test/test3.gss");
    assert!(s.is_ok(), "{s:?}");
    let s = s.unwrap();
    s.dump(0);
    println!("test.hex as i32: {:?}", s.get_as::<i32>(&["test", "hex"]));
}
