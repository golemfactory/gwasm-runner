# GWasm Runner Tutorial

# Summary
This tutorial will guide you through following steps:

0. Setup Golem Unlimited cluster
   1. Install
   2. Clone & Build
   3. Start GU Hub
   4. Configure and Start GU Provider
1. Setup Development Environment
   1. Standalone
      * Rust
      * Emscripten      
      * VCPKG (Windows)
      * OpenSSL
      * gwasm-runner
      * Sample projects
        * are they in one repo???
   2. Docker-based
2. View Mandelbrot
3. Build Mandelbrot
4. Run Mandelbrot via gwasm-runner locally
5. Run Mandelbrot via gwasm-runner on GU cluster
6. View results

# 0. Setup Golem Unlimited Cluster

## i. Install

## ii. Clone & Build

## iii. Start GU Hub

## iv. Configure and Start GU Provider

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

`docker pull prekucki/gwasm-tutoral`

`docker run -it prekucki/gwasm-tutoral`

(or with option to mount a local file share)

`docker run -it -v output:/root/output prekucki/gwasm-tutoral`

## 2. View Mandelbrot

`cd ..\mandelbrot`

`ls`

`cd src`

`vi mandelbrot.rs`

Observe the structure of the split() -> execute() -> merge() pattern which is leveraged by gwasm-runner.

## 3. Build Mandelbrot

`cargo build --release --target wasm32-unknown-emscripten`

## 4. Run Mandelbrot via gwasm-runner locally

`gwasm-runner ./target/wasm32-unknown-emscripten/release/mandelbrot.wasm 10 10 10 10 10 10 2`

## 5. Run Mandelbrot via gwasm-runner on GU cluster

`gwasm-runner ???`

## 6. View results

Observe the output file in the `mandelbrot` directory:

`ls`

Now we need to move the output file (`out.png`) to the output directory which is shared with the host:

`cp ./out.png ../host`

At this point, the `out.png` file should be visible in host filesystem, and viewable with your preferred image viewer.
