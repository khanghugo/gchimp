# gchimp-web

This crate will re-export `gchimp` modules for `wasm32-unknown-unknown` and then do some bindgen stuffs for `gchimp-web-www` to use.

## Building

You need `wasm-pack` to nicely package `gchimp-web`. Refer to your OS package manager to get it.

Then,

```sh
wasm-pack build
```

You then will have a `pkg` folder inside `gchimp-web`. It will be a NodeJS module that you can use it for a NodeJS project.
