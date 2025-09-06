mod formats;
mod parser;
pub mod types;

pub use parser::parse_vtf;
pub use types::Vtf;

use types::*;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse1() {
        let vtf_bytes = include_bytes!("tests/prodcaution_green.vtf");
        let vtf = Vtf::from_bytes(vtf_bytes).unwrap();

        assert_eq!((vtf.header.width, vtf.header.height), (256, 256));
    }

    #[test]
    fn parse2() {
        let vtf_bytes = include_bytes!("tests/dev_measuregrid.vtf");
        let vtf = Vtf::from_bytes(vtf_bytes).unwrap();

        assert_eq!((vtf.header.width, vtf.header.height), (512, 512));
    }
}
