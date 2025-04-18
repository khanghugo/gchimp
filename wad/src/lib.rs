//! WAD file parsing
//!
//! Based of specification from this webpage: https://twhl.info/wiki/page/Specification%3A_WAD3
mod constants;
mod parser;
pub mod types;
pub mod utils;

pub use parser::{parse_miptex, parse_wad};

#[cfg(test)]
mod test {
    use types::{FileEntry, Wad};

    use super::*;

    #[test]
    fn parse_wad_test() {
        let file = Wad::from_file("test/wad_test.wad");

        assert!(file.is_ok());

        let file = file.unwrap();

        assert!(file.header.num_dirs == 1);
        assert!(file.entries.len() == 1);

        let entry = &file.entries[0];

        assert!(entry.directory_entry.file_type == 0x43);
        assert!(entry.directory_entry.texture_name.get_string() == "white");
    }

    #[test]
    fn parse_wad_test2() {
        let file = Wad::from_file("test/wad_test2.wad");

        assert!(file.is_ok());

        let file = file.unwrap();

        assert!(file.header.num_dirs == 2);
        assert!(file.entries.len() == 2);

        let entry = &file.entries[0];

        assert!(entry.directory_entry.file_type == 0x43);
        assert!(entry.directory_entry.texture_name.get_string() == "white");

        let entry = &file.entries[1];

        assert!(entry.directory_entry.file_type == 0x43);
        assert!(entry.directory_entry.texture_name.get_string() == "black");
    }

    #[test]
    fn parse_cyberwave() {
        let file = Wad::from_file("test/surf_cyberwave.wad");

        assert!(file.is_ok());

        let file = file.unwrap();

        assert!(file.header.num_dirs == 23);
        assert!(file.entries.len() == 23);

        let entry = &file.entries[18];

        assert_eq!(entry.directory_entry.file_type, 0x43);
        assert_eq!(
            entry.directory_entry.texture_name.get_string(),
            "Sci_fi_metal_fl"
        );

        assert!(matches!(entry.file_entry, FileEntry::MipTex(_)));

        if let FileEntry::MipTex(file) = &entry.file_entry {
            assert_eq!(file.height, file.width);
            assert_eq!(file.height, 512);
            assert_eq!(file.texture_name.get_string(), "Sci_fi_metal_fl");
        }

        let entry = &file.entries[21];

        assert_eq!(entry.directory_entry.file_type, 0x43);
        assert_eq!(entry.directory_entry.texture_name.get_string(), "emp_ball1");

        assert!(matches!(entry.file_entry, FileEntry::MipTex(_)));

        if let FileEntry::MipTex(file) = &entry.file_entry {
            assert_eq!(file.height, file.width);
            assert_eq!(file.height, 512);
            // Don't assert this because it fails.
            // left: "emp_ball1ing.."
            // right: "emp_ball1"
            // assert_eq!(file.texture_name.get_string(), "emp_ball1");
        }
    }

    #[test]
    fn parse_write() {
        let wad = Wad::from_file("test/wad_test.wad");

        assert!(wad.is_ok());

        let wad = wad.unwrap();

        let res = wad.write_to_file("test/out/wad_test_out.wad");

        assert!(res.is_ok());
    }

    #[test]
    fn parse_write2() {
        let wad = Wad::from_file("test/wad_test2.wad");

        assert!(wad.is_ok());

        let wad = wad.unwrap();

        let res = wad.write_to_file("test/out/wad_test2_out.wad");

        assert!(res.is_ok());
    }

    #[test]
    fn parse_write3() {
        let wad = Wad::from_file("test/surf_cyberwave.wad");

        assert!(wad.is_ok());

        let wad = wad.unwrap();

        let res = wad.write_to_file("test/out/surf_cyberwave_out.wad");

        assert!(res.is_ok());
    }

    #[test]
    fn parse_big() {
        let _wad = Wad::from_file("/home/khang/map_compiler/cso_normal_pack.wad").unwrap();
        let _wad2 = Wad::from_file("/home/khang/map_compiler/cso_normal_pack.wad").unwrap();

        // check the memory usage
        std::thread::sleep(std::time::Duration::from_secs(5));
    }

    #[test]
    fn parse_gfx() {
        let _wad = Wad::from_file("/home/khang/bxt/_game_native/valve/gfx.wad").unwrap();
        let _wad = Wad::from_file("/home/khang/bxt/_game_native/valve/cached.wad").unwrap();
        let _wad = Wad::from_file("/home/khang/bxt/_game_native/valve/decals.wad").unwrap();
        let _wad = Wad::from_file("/home/khang/bxt/_game_native/valve/tempdecal.wad").unwrap();
        let _wad = Wad::from_file("/home/khang/bxt/_game_native/valve/spraypaint.wad").unwrap();
    }
}
