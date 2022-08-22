pub mod c_table;
pub mod w_table;
pub(crate) mod utils;

pub use c_table::{CTable, CompleteGrouping, make_c_table, c_entry_iter};
pub use w_table::{WTable, WaitingPattern, make_w_table, w_entry_iter};

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::OnceCell;

    fn get_c_table() -> &'static CTable {
        static C_TABLE: OnceCell<CTable> = OnceCell::new();
        C_TABLE.get_or_init(make_c_table)
    }

    fn get_w_table() -> &'static WTable {
        static W_TABLE: OnceCell<WTable> = OnceCell::new();
        W_TABLE.get_or_init(|| make_w_table(get_c_table()))
    }

    #[test]
    fn check_num_keys() {
        let c_table = get_c_table();
        let w_table = get_w_table();
        assert_eq!(c_table.len(), c_table::C_TABLE_NUM_KEYS);
        assert_eq!(w_table.len(), w_table::W_TABLE_NUM_KEYS);
    }

    #[test]
    fn abc() {

    }
}
