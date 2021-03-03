# Physics based tetris clone

* Based on [@audunhalland](https://github.com/audunhalland)'s [workshop](https://github.com/audunhalland/bevy-tetris-workshop)
* Built with [Bevy](https://bevyengine.org/) and [Rapier](https://rapier.rs/)
* Muti target: WASM and native.

## Prerequisites
Install cargo-make:

```
cargo install cargo-make
```

## Development

This repo uses the Cargo Makefile shamelessly stolen from [ mrk-its /
bevy_webgl2_app_template ](https://github.com/mrk-its/bevy_webgl2_app_template)

### WASM
Build and serve:

```
cargo make serve
```
then point your browser to http://127.0.0.1:4000/

Only build, no serve:
```
cargo make build-web
```


## Native
Build and run:
```
cargo make run
```

Build binary:
```
cargo make build-native
``` 
