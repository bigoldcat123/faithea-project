#[test]
fn a() {
    let a = "/a/";
    for i in a.split("/") {
        println!("-> [{}]",i);
    }
}
