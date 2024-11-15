# gchimp-web-www

This is a webapp for gchimp with limited features.

## Pre-requisite

You must first have `gchimp` compiled as a WASM module. In order to do that, you should refer to [gchimp-web README](../README.md).

## Developing

Just

```sh
npm run dev
```

and read the output.

## Deploying

After having `gchimp` compiled into a WASM module, you should have `pkg` folder inside `gchimp-web` which contains the NodeJS module.

1. Change directory to `/gchimp-web/www` and then

    ```sh
    npm run build
    ```

2. After building it, run this to deploy

    ```sh
    npm run start
    ```
