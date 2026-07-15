#[cfg(test)]
mod test {
    use common::img_stuffs::rgba8_to_8bpp;

    use crate::StudioMdl;

    #[test]
    fn simple_tri() {
        let mut studiomdl = StudioMdl::new();

        let smd_text = include_str!("./test.smd");
        let image_bytes = include_bytes!("./texture.bmp");
        let image = image::load_from_memory(image_bytes.as_slice()).unwrap();

        let smd = smd::Smd::from(smd_text).unwrap();
        let texture = rgba8_to_8bpp(image.to_rgba8()).unwrap();

        smd.triangles.iter().for_each(|tri| {
            studiomdl.add_triangle(tri.clone());
        });

        studiomdl.add_texture(("texture.bmp", texture, mdl::TextureFlag::FLATSHADE));

        studiomdl.set_model_name("test_syn.mdl");

        let res = studiomdl.compile().unwrap();

        let res_bytes = res.write_to_bytes();
        let syn = mdl::Mdl::open_from_bytes(&res_bytes).unwrap();

        println!("{:?}", syn);

        let gt_bytes = include_bytes!("./test_gt.mdl");
        let gt = mdl::Mdl::open_from_bytes(gt_bytes).unwrap();
        println!("{:?}", gt);
        res.write_to_file("./src/tests/test_syn.mdl").unwrap();
    }
}
