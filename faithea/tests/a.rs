use std::path::PathBuf;

use faithea::TryConvertInto;
use serde::{Deserialize, Serialize};


#[test]
fn u() {

    let mask = [ 0x8d, 0x20, 0xe3, 0xdf];
    let value = [0xf6, 0x02, 0x97, 0xa6, 0xfd, 0x45, 0xc1, 0xe5, 0xaf, 0x4d, 0x86, 0xac, 0xfe, 0x41, 0x84, 0xba, 0xaf, 0x0c, 0xc1, 0xab, 0xe2, 0x02, 0xd9, 0xfd, 0xec, 0x02, 0xcf, 0xfd, 0xeb, 0x52, 0x8c, 0xb2, 0xaf, 0x1a, 0xc1, 0xbe, 0xaf, 0x0c, 0xc1, 0xbc, 0xe2, 0x4e, 0x97, 0xba, 0xe3, 0x54, 0xc1, 0xe5, 0xaf, 0x41, 0x90, 0xbb, 0xaf, 0x5d];
    println!("lent: {}", value.len());

    for i in 0..value.len() {
        let masked = (mask[i % 4] ^ value[i]) as u8;
        println!("masked: {}", masked as char);
    }

}

#[test]
fn a() {
    let a = "/a/";
    for i in a.split("/") {
        println!("-> [{}]", i);
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Person {
    name: String,
    age: i32,
}

#[test]
fn t2() {
    let p = Person {
        name: "asd".into(),
        age: 22,
    };
    let s = serde_json::to_string_pretty(&p).unwrap();
    println!("{}", s);
    let a = serde_json::from_str::<Person>(s.as_str());
    println!("{:?}", a);
}

#[test]
fn t3() {
    fn a(_p1: usize, _p2: &str, _p3: &String, _p4: &String, _p5: i32) {}
    let p = &"2".to_string();
    a(
        p.try_convert_into().map_err(|_| "").unwrap(),
        p.try_convert_into().map_err(|_| "").unwrap(),
        p.try_convert_into().map_err(|_| "").unwrap(),
        p.try_convert_into().map_err(|_| "").unwrap(),
        p.try_convert_into().map_err(|_| "").unwrap(),
    );
}

#[test]
fn path() {
    let a = PathBuf::from("/Users/dadigua/Desktop/graduation/http-server/src/data/inbound");
    println!("{:?}", a.extension());
}

#[test]
fn b() {

}
