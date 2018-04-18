fn main() {
    let s = "  http://a.org  , http://b.org,   ";

    let v: Vec<_> = s.split(&[' ', ','][..]).filter(|s| !s.is_empty()).collect();

    println!("{:?}", v);
}
