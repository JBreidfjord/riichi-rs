pub mod c_table;
pub mod w_table;
pub mod utils;

pub use c_table::{CTable, make_c_table};
pub use w_table::{WTable, WEntry, make_w_table};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_num_keys() {
        let c_table = make_c_table();
        let w_table = make_w_table(&c_table);
        assert_eq!(c_table.len(), c_table::C_TABLE_NUM_KEYS);
        assert_eq!(w_table.len(), w_table::W_TABLE_NUM_KEYS);
    }
}
