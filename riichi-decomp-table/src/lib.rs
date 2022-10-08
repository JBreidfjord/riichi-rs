#[doc = include_str!("../README.md")]

pub mod c_table;
pub mod w_table;
pub mod utils;

pub use c_table::*;
pub use w_table::*;

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
        assert_eq!(c_table.len(), C_TABLE_NUM_KEYS);
        assert_eq!(w_table.len(), W_TABLE_NUM_KEYS);
    }

    #[test]
    fn check_c_trivial_entries() {
        let c_table = &C_TABLE;

        let key = 0o000020000;
        let &value = c_table.get(&key).unwrap();
        let result = c_entry_iter_alts(key, value).collect_vec();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].groups.len(), 0);
        assert_eq!(result[0].pair(), Some(4));

        assert_eq!(c_table[&0], CAlts::new().with(!0));
        let zero_result = c_entry_iter_alts(0, CAlts::new().with(!0)).collect_vec();
        assert_eq!(zero_result.len(), 1);
        assert_eq!(zero_result[0].groups.len(), 0);
        assert_eq!(zero_result[0].pair(), None);

        assert_eq!(c_table[&0o000000003], CAlts::new().with(!0x0000));
        assert_eq!(c_table[&0o300000000], CAlts::new().with(!0x000F));
    }
}
