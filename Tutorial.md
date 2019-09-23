# GWasm Runner Tutorial

# Summary
This tutorial will guide you through following steps:

0. Setup Golem Unlimited cluster
   1. Install
   2. Clone & Build
   3. Start GU Hub
   4. Configure and Start GU Provider
1. Setup Development Environment
   1. Docker-based
   2. Standalone
2. Hello example
3. Mandelbrot example
4. Other examples

# 0. Setup Golem Unlimited Cluster

## i. Install

TODO: Links to install package downloads.

## ii. Clone & Build

TODO: links and command lines to clone golem-unlimited repo and build the modules.

## iii. Start GU Hub

`<commandline to start GU Hub>`

URL of the UI Console:

`http://localhost:61622/app/index.html`

## iv. Configure and Start GU Provider

`<commandline to configure the Provider node>`

`<commandline to start Provider node>`

You should now be able to observe the new GU Provider appearing in the GU Hub UI Console.

# 1. Setup Development Environment

## i. Standalone

### Prerequisites
  * Rust
  * VCPKG (Windows)
  * OpenSSL
  * gwasm-runner
  * Sample projects
    * are they in one repo???

## ii. Docker-based

`docker pull golemfactory/gwasm-tutorial`

or, if using local Deocker Registry:

`docker pull <registry IP>/golemfactory/gwasm-tutorial`

`mkdir gwasm-tutorial-workspace`

`docker run -it -v $(pwd)/gwasm-tutorial-workspace:/data golemfactory/gwasm-tutorial`

or

`docker run -it -v $(pwd)/gwasm-tutorial-workspace:/data <registry IP>/golemfactory/gwasm-tutorial`


## 2. Hello example
### Code

`vi \root\hello\src\hello.rs`

Observe the structure of the split() -> execute() -> merge() pattern which is leveraged by gwasm-runner.

### Build

`cargo build --target=wasm32-unknown-emscripten --release`

### Run Hello via gwasm-runner locally

`gwasm-runner ./target/wasm32-unknown-emscripten/release/hello.wasm`

### Run Hello via gwasm-runner on GU cluster

`gwasm-runner ???`

## 3. Mandlebrot example
### Code

`vi \root\mandelbrot\src\mandelbrot.rs`

Observe the structure of the split() -> execute() -> merge() pattern which is leveraged by gwasm-runner.

### Build

`cargo build --target=wasm32-unknown-emscripten --release`

### Run Mandelbrot via gwasm-runner locally

`gwasm-runner ./target/wasm32-unknown-emscripten/release/mandelbrot.wasm 0.2 0.35 0.6 0.45 1000 1000 2`

### Run Mandelbrot via gwasm-runner on GU cluster

`gwasm-runner ???`

### View results

Observe the output file in the `mandelbrot` directory:

`ls`

Now we need to move the output file (`out.png`) to the output directory which is shared with the host:

`cp /root/mandelbrot/out.png /data/mandelbrot.png`

At this point, the `out.png` file should be visible in host filesystem, and viewable with your preferred image viewer.

## 4. Other examples

In the developer's Image you can find other examples of gWasm projects:
  - /hello - The Hello World example, presented in this tutorial,
  - /mandelbrot - The Mandelbrot example, presented in this tutorial,
  - /aes-cracker - A *very* interesting example, which includes a *prize* for those worthy explorers who are willing to join their forces and break the code... ;)
