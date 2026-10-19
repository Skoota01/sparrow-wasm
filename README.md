# sparrow-wasm

Browser build of [Sparrow](https://github.com/JeroenGar/sparrow) (MIT) for the Crossfire Nester app.
`sparrow/` is a copy of Sparrow with one change: on wasm32 the separator workers run one after another
instead of through rayon threads. `web/` is the wasm-bindgen wrapper. GitHub Actions builds it and
pushes `sparrow_web.js` and `sparrow_web_bg.wasm` to the `build-output` branch.
