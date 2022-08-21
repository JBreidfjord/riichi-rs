use std::fs::File;
use std::io;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use itertools::Itertools;

use riichi_decomp_table::*;

fn main() -> io::Result<()> {
    let t1 = std::time::Instant::now();
    let c_table = make_c_table();
    let t2 = std::time::Instant::now();
    let dt_c = t2.duration_since(t1);
    let w_table = make_w_table(&c_table);
    let t3 = std::time::Instant::now();
    let dt_w = t3.duration_since(t2);

    println!("C table: {} us", dt_c.as_micros());
    println!("W table: {} us", dt_w.as_micros());
    println!("W table len: {}", w_table.len());

    BufWriter::new(File::create(PathBuf::from("../../../c.txt"))?)
        .write_all(c_table::table_to_string(&c_table).as_bytes())?;

    let mut c_map_gen = phf_codegen::Map::<u32>::new();
    for (k, v) in c_table.iter() {
        c_map_gen.entry(*k, &format!("{}u64", v));
    }
    let mut map_file = BufWriter::new(File::create(PathBuf::from("../../../c.rs"))?);
    write!(map_file,
           "static C_TABLE: phf::Map<u32, u64> = \n{};\n",
           c_map_gen.build())?;

    /*
    let (key, v) =
        w_table.iter().max_by_key(|(k, v)| v.len()).unwrap();
    println!("max len: W[{:09o}] = {}", key, v.len());
    for entry in v.iter() {
        println!("{:?}", *entry);
    }
    */

    Ok(())
}
