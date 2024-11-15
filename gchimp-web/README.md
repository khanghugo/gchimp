# gchimp-web

This crate will re-export `gchimp` modules for `wasm32-unknown-unknown` and then do some bindgen stuffs for `gchimp-web-www` to use.

## Building

```sh
wasm-pack build
```

You then will have a `pkg` folder inside `gchimp-web`. It will be a NodeJS module that you can use it for a NodeJS project.
