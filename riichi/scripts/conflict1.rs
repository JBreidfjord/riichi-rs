#!/usr/bin/env rust-script
// cargo-deps: serde = "1.0", serde_json = "1.0", num_enum="0.5.7"

use serde::{Serialize, Deserialize};
use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Copy, Clone, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum MyEnum {
    A = 0,
    B = 1,
    C = 2,
    D = 3,
}

pub fn main() {
    let s = serde_json::to_string(&12345i32);
    println!("{}", s.unwrap());
    // println!("{}", 3u8 == MyEnum::D.into());
    println!("{}", 3u8 == u8::from(MyEnum::D));
}
