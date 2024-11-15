# gchimp-web-www

This is a webapp for gchimp with limited features.

## Developing

Just

```sh
npm run dev
```

and read the output.

## Deploying

1. You must compile WASM binary with `--target=wasm32-unknown-unknown`. To do this nicely, just install `wasm-pack` and then

    ```sh
    wasm-pack build
    ```

    inside `gchimp-web` package.

    From then on, `gchimp-web-www` will link to `/gchimp-web/pkg` module

2. Change directory to `gchimp-web-www` and then

    ```sh
    npm run build
    ```

3. After building it, run this to deploy

    ```sh
    npm run start
    ```
