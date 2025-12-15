use std::io::stdin;
/// #conclusion
/// when the memory is too big , it will release when it's droppd
/// when the memory is not big , it will not realease when it's dropped
fn main() {
    let mut buf = String::new();
    {
        // 4
        let mut a = vec![0_i32;1024*1024*500];
        a.fill(1000);
        let _  = stdin().read_line(&mut buf);
        // println!("{:?}",a);
//
    }
    let _  = stdin().read_line(&mut buf);
    println!("{}",buf);

}
