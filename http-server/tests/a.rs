
use std::path::PathBuf;

use http_server::request::ConvertFromRefString;
use serde::{Deserialize, Serialize};

#[test]
fn a() {
    let a = "/a/";
    for i in a.split("/") {
        println!("-> [{}]",i);
    }
}

#[derive(Serialize,Deserialize,Debug)]
struct Person {
    name:String,
    age:i32
}

#[test]
fn t2() {
    let p = Person{name:"asd".into(),age:22};
    let s = serde_json::to_string_pretty(&p).unwrap();
    println!("{}",s);
    let a = serde_json::from_str::<Person>(s.as_str());
    println!("{:?}",a);

}


#[test]
fn t3() {
    fn a(_p1:usize,_p2:&str,_p3:&String,_p4:&String,_p5:i32) {

    }
    let p = &"2".to_string();
    a(p.convert().unwrap(),p.convert().unwrap(),p.convert().unwrap(),p.convert().unwrap(),p.convert().unwrap());
}



#[test]
fn path() {
    let a  = PathBuf::from("/Users/dadigua/Desktop/graduation/http-server/src/data/inbound");
    println!("{:?}",a.extension());




}
