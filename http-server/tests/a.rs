#[test]
fn a() {
    let a = "a/b/c/d/";
    for i in a.split("/") {
        println!("{}",i);
    }
}
