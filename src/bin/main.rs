use gss::load_gss_from_file;

fn main() {
    let s = load_gss_from_file("test/test.gss");
    assert!(s.is_ok(), "{s:?}");
    let s = s.unwrap();
    s.dump(0);
    println!("{:?}", s.get::<i32>(&["style", "top"]));
    println!("{:?}", s.get::<String>(&["style", "inner", "link"]));
}
