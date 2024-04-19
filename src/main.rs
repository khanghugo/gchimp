mod cli;
mod modules;

// #![allow(non_snake_case)]

use dioxus::prelude::*;
use log::LevelFilter;

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
    #[route("/blog/:id")]
    Blog { id: i32 },
}

fn main() {
    // Init debug
    // dioxus_logger::init(LevelFilter::Info).expect("failed to init logger");

    // dioxus::launch(App);
    cli::cli();
}

#[component]
fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

#[component]
fn Blog(id: i32) -> Element {
    rsx! {
        Link { to: Route::Home {}, "Go to counter" }
        "Blog post {id}"
    }
}

#[component]
fn Home() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        Link {
            to: Route::Blog {
                id: count()
            },
            "Go to blog"
        }
        div {
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
        }
    }
}

// fn main() {
//     // main_cli();

//     let mut map = map::Map::new("./examples/surf_raphaello.map");
//     map.light_scale((1., 1., 1., 1.));
//     // texture_scale::texture_scale(&mut map, 16.);
//     // rotate_prop_static::rotate_prop_static(&mut map, Some("remec_lit_model"));
//     // light_scale::light_scale(&mut map, (1., 1., 1., 0.25));

//     // map.write("./examples/out.map").unwrap();
// }
