use bytes::{Buf, BufMut};

#[test]
fn a() {
    let mut a = bytes::BytesMut::new();
    a.put("\r\n\r\n".as_bytes());
    println!("{:#?}", a);

    let b = a.get_u32();
    println!("{:X} {b}", b);
    println!("{:#?}", a);
    asdasd
}
