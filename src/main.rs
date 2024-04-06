use map::{self, Map};

mod rotate_prop_static;
mod texture_scaler;

fn main() {
    let mut map = Map::new("./examples/surf_raphaello.map");
    texture_scaler::texture_scaler(&mut map, 16.);
    rotate_prop_static::rotate_prop_static(&mut map, Some("remec_lit_model"));

    map.write("./examples/out.map").unwrap();
}
