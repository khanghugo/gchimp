pub mod error;
mod parser;
mod types;
mod utils;
mod writer;

pub use types::*;

#[cfg(test)]
mod test {
    use crate::Spr;

    #[test]
    fn parse_glow() {
        let file = include_bytes!("../test/glow01.spr");
        let spr = Spr::open_from_bytes(file).unwrap();

        let image = spr.to_rgb8(0);
        // image.save("./test/glow01.png").unwrap();
    }

    #[test]
    fn parse_hud() {
        let file = include_bytes!("../test/640hud1.spr");
        let spr = Spr::open_from_bytes(file).unwrap();

        let image = spr.to_rgb8(0);
        // image.save("./test/640hud1.png").unwrap();
    }

    #[test]
    fn parse_many_frames() {
        let file = include_bytes!("../test/d-tele1.spr");
        let spr = Spr::open_from_bytes(file).unwrap();

        let image = spr.to_rgb8(0);
        // image.save("./test/d-tele1_0.png").unwrap();

        let image = spr.to_rgb8(10);
        // image.save("./test/d-tele1_1.png").unwrap();

        let image = spr.to_rgb8(20);
        // image.save("./test/d-tele1_3.png").unwrap();
    }

    #[test]
    fn parse_write_parse() {
        let file = include_bytes!("../test/d-tele1.spr");
        let spr1 = Spr::open_from_bytes(file).unwrap();
        let file2 = spr1.write_to_bytes();
        let spr2 = Spr::open_from_bytes(&file2).unwrap();

        assert_eq!(spr1.frames.len(), spr1.frames.len());
        assert_eq!(spr1.palette, spr2.palette);
        assert_eq!(spr1.header, spr2.header);

        spr1.frames.iter().zip(spr2.frames).for_each(|(f1, f2)| {
            assert_eq!(f1.header, f2.header);
            assert_eq!(f1.image, f2.image);
        });
    }
}
