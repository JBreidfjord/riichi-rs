pub mod c_table;
pub mod w_table;
pub mod utils;

pub use c_table::{CTable, CTableStatic, CompleteGrouping, make_c_table, c_entry_iter};
pub use w_table::{WTable, WTableStatic, WaitingPattern, WaitingKind, make_w_table, w_entry_iter};

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use once_cell::sync::Lazy;

    static C_TABLE: Lazy<CTable> = Lazy::new(make_c_table);
    static W_TABLE: Lazy<WTable> = Lazy::new(|| make_w_table(&C_TABLE));

    #[test]
    fn check_num_keys() {
        let c_table = &C_TABLE;
        let w_table = &W_TABLE;
        assert_eq!(c_table.len(), c_table::C_TABLE_NUM_KEYS);
        assert_eq!(w_table.len(), w_table::W_TABLE_NUM_KEYS);
    }

    #[test]
    fn check_c_trivial_entries() {
        let c_table = &C_TABLE;

        let key = 0o000020000;
        let &value = c_table.get(&key).unwrap();
        let result = c_entry_iter(key, value).collect_vec();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].num_groups, 0);
        assert_eq!(result[0].pair(), Some(4));

        assert_eq!(c_table[&0], 0);
        let zero_result = c_entry_iter(0, 0).collect_vec();
        assert_eq!(zero_result.len(), 1);
        assert_eq!(zero_result[0].num_groups, 0);
        assert_eq!(zero_result[0].pair(), None);
    }
}
