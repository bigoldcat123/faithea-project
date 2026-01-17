use std::path::PathBuf;

use faithea::{TryConvertFrom, TryConvertInto};
use serde::{Deserialize, Serialize};


#[test]
fn base64() {

    struct A {

    }
    struct B {

    }
    impl TryConvertFrom<A> for  B {
        fn try_convert_from(value: A) -> Result<Self, faithea::handler::types::HttpHandlerError> {
            Ok(B{})
        }
    }
    let b:B = A{}.try_convert_into().unwrap();

}

#[test]
fn u() {

    let mask = [0x01, 0x02, 0x03, 0x04];
    let value = b"hello";
    println!("lent: {}", value.len());

    for i in 0..value.len() {
        let masked = (mask[i % 4] ^ value[i]) as u8;
        println!("masked: {}", masked);
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
