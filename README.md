# gchimp

A collection of various GoldSrc mapping tools with both graphical and command line interface

![default](./docs/screenshot_s2g_populated.png)

## Installation

Head to [Release](https://github.com/khanghugo/gchimp/releases) page to download the app or latest [Actions](https://github.com/khanghugo/gchimp/actions) commits.

Usually it would just work out of the box. If there is any problem, try starting the binary through terminal and you will see the program output to better diagnose the issue.

## Features

### S2G

Source to GoldSrc models converter. Supports folder conversion.

### SkyMod

Creates skybox as a model. Supports texture size bigger than the typical 512x512.

### Command line interface

Some functionalities have command line as well as graphical supports. There is also .rhai scripting for a selected few functionalities.

### QC, SMD, MAP

You can write your own functionalities with these included libraries.

### Planned features

- [ ] A functional enough Wally to edit WAD files 
- [ ] [map2prop](https://erty-gamedev.github.io/Docs-Map2Prop/) clone that hopefully goes open source before the original project

## Build

After building the project, put `config.toml` from `dist` folder into the same folder as the binary `gchimp`

Then, download [no_vtf](https://sr.ht/~b5327157/no_vtf/) and put the `no_vtf` folder inside `dist`
