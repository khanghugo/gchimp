#[cfg(test)]
mod test {
    use crate::StudioMdl;

    fn simple_tri() {
        let studiomdl = StudioMdl::new();
        let smd_text = include_str!("./test.smd");
        let smd = smd::Smd::from(smd_text).unwrap();
        // let texture =
    }
}
