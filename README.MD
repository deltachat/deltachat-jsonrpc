## Build Requirements

- Linux or Mac, scrips make use of features like `>` pipes and `&&` (maybe the newer versions of powershell support them, but I didn't try that.)
- rust (installed via rustup)

## Start the webserver

The webserver is an example usage. Goal of it is to be usable both as example and as base for deltachat-kaiOS.

```sh
RUST_LOG=info cargo run --features webserver
```

## Generate Typescript Bindings

```sh
cd typescript
npm i
npm run build
```

## Run the development example

Mac

```sh
alias firefox=/Applications/Firefox.app/Contents/MacOS/firefox
npm run example:build && firefox --devtools $(pwd)/example/browser-example.html
```

Linux:

```sh
npm run example:run
```

## Compiling server for kaiOS or android:

```sh
cross build --features=webserver --target armv7-linux-androideabi --release
```
