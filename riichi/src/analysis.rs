pub mod decomp;
pub mod irregular;

pub use decomp::{Decomposer, RegularWait};
pub use irregular::{IrregularWait, detect_irregular_wait};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Wait {
    Regular(RegularWait),
    Irregular(IrregularWait),
}
