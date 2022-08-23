use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use itertools::Itertools;

use riichi_decomp_table::{CTable, WTable, make_c_table, make_w_table};
use riichi::analysis::decomp::w_table_naive as w_table;

fn main() {
    /*
    let c_table = c_table::make_complete();
    let w_table = w_table::make_waiting(&c_table);

    let mut c_map_gen = phf_codegen::Map::<u32>::new();
    for (k, v) in c_table.iter() {
        c_map_gen.entry(*k, &format!("{}u64", v));
    }

     */

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("decomp_tables.rs");

    let mut map_file = BufWriter::new(File::create(dest_path).unwrap());
    /*
    write!(map_file,
           "static C_TABLE: phf::Map<u32, u64> = \n{};\n",
           c_map_gen.build());
     */
    write!(map_file, "pub const GENERATED: i32 = 114514;");

    println!("cargo:rerun-if-changed=_build");
}
