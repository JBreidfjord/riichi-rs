#!/usr/bin/env rust-script
// cargo-deps: ux="0.1.5"
use ux::{i2, u2};

fn main() {
    println!("{}", u2::new(1) - u2::new(3));
}
