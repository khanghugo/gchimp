mod cli;
mod gui;
mod modules;

fn main() {
    gui::gui();
    // cli::cli();

    // let mut map = map::Map::new("./examples/surf_raphaello.map");
    // map.light_scale((1., 1., 1., 1.));
    // texture_scale::texture_scale(&mut map, 16.);
    // rotate_prop_static::rotate_prop_static(&mut map, Some("remec_lit_model"));
    // light_scale::light_scale(&mut map, (1., 1., 1., 0.25));

    // map.write("./examples/out.map").unwrap();
}
