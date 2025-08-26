pub mod error;
mod parser;
mod types;
mod utils;

pub use types::*;

#[cfg(test)]
mod test {
    use crate::Spr;

    #[test]
    fn parse_glow() {
        let file = include_bytes!("../test/glow01.spr");
        let spr = Spr::open_from_bytes(file).unwrap();

        let image = spr.to_rgb8(0);
        image.save("./test/glow01.png").unwrap();
    }

    #[test]
    fn parse_hud() {
        let file = include_bytes!("../test/640hud1.spr");
        let spr = Spr::open_from_bytes(file).unwrap();

        let image = spr.to_rgb8(0);
        image.save("./test/640hud1.png").unwrap();
    }

    #[test]
    fn parse_many_frames() {
        let file = include_bytes!("../test/d-tele1.spr");
        let spr = Spr::open_from_bytes(file).unwrap();

        let image = spr.to_rgb8(0);
        image.save("./test/d-tele1_0.png").unwrap();

        let image = spr.to_rgb8(10);
        image.save("./test/d-tele1_1.png").unwrap();

        let image = spr.to_rgb8(20);
        image.save("./test/d-tele1_3.png").unwrap();
    }
}
