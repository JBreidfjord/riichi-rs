use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path};

use riichi_decomp_table::{make_c_table, make_w_table};

fn main() {
    let c_table = make_c_table();
    let w_table = make_w_table(&c_table);

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("decomp_tables.rs");

    if dest_path.exists() { return; }
    let mut map_file = BufWriter::new(File::create(dest_path).unwrap());
    {
        let mut c_map_gen = phf_codegen::Map::<u32>::new();
        for (k, v) in c_table.iter() {
            c_map_gen.entry(*k, &format!("{}u64", v));
        }
        write!(map_file,
               "static C_TABLE: phf::Map<u32, u64> = \n{};\n",
               c_map_gen.build()).unwrap();
    }
    {
        let mut w_map_gen = phf_codegen::Map::<u32>::new();
        for (k, v) in w_table.iter() {
            w_map_gen.entry(*k, &format!("{}u64", v));
        }
        write!(map_file,
               "static W_TABLE: phf::Map<u32, u64> = \n{};\n",
               w_map_gen.build()).unwrap();
    }

    println!("cargo:rerun-if-changed=build");
}
