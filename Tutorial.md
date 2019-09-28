# GWasm Runner Tutorial

With this tutorial, we hope to get you acquainted with gWasm, basics of Golem Unlimited,
and how to use our latest `gwasm-runner` helper API for easily running your gWasm apps
locally, on Golem Unlimited (GU) cluster, and Golem Brass (Brass) (coming soon!). We've organised
this tutorial into several sections as shown in the table of contents below.

At our Devcon5 workshop, we will go through the majority of described steps in this
tutorial, so you can either follow it with us on the big screen, or feel free to
to do it yourself following this written tutorial.

# Table of Contents
- [0. Setup Golem Unlimited cluster](#0.-setup-golem-unlimited-cluster)
  - [i. Install](#i.-install)
  - [ii. Clone & Build](#ii.-clone-%26-build)
  - [iii. Start GU Hub](#iii.-start-gu-hub)
  - [iv. Configure and Start GU Provider](#iv.-configure-and-start-gu-provider)
- [1. Setup Development Environment](#1.-setup-development-environment)
  - [i. Docker-based](#i.-docker-based)
  - [ii. Standalone](#ii.-standalone)
    - [a. Emscripten SDK](#a.-emscripten-sdk)
    - [b. Rust](#b.-rust)
    - [c. OpenSSL and other dependencies](#c.-openssl-and-other-dependencies)
    - [d. gwasm-runner](#d.-gwasm-runner)
- [2. Hello World! example](#2.-hello-world!-example)
  - [i. The gWasm runner API](#i.-the-gwasm-runner-api)
  - [ii. Build](#ii.-build)
  - [iii. Run!](#iii.-run!)
- [3. Mandelbrot example](#3.-mandelbrot-example)
- [4. Other examples](#4.-other-examples)

# 0. Setup Golem Unlimited Cluster

## i. Install

TODO: Links to install package downloads.

## ii. Clone & Build

TODO: links and command lines to clone golem-unlimited repo and build the modules.

## iii. Start GU Hub

To run the hub from source, in the `golem-unlimited` folder structure please go to the `gu-hub` subdir and perform:
```
$ cargo run -- -vv server run
```

URL of the UI Console:

`http://localhost:61622/app/index.html`

## iv. Configure and Start GU Provider

`<commandline to configure the Provider node>`

`<commandline to start Provider node>`

You should now be able to observe the new GU Provider appearing in the GU Hub UI Console.

# 1. Setup Development Environment

## i. Docker-based

`docker pull golemfactory/gwasm-tutorial`

or, if using local Docker Registry:

`docker pull docker.golem.network:5000/golemfactory/gwasm-tutorial`

`mkdir gwasm-tutorial-workspace`

`docker run -it -v $(pwd)/gwasm-tutorial-workspace:/data golemfactory/gwasm-tutorial`

or

`docker run -it -v $(pwd)/gwasm-tutorial-workspace:/data docker.golem.network:5000/golemfactory/gwasm-tutorial`

## ii. Standalone

If you would like to cross-compile your apps to gWasm, you need to have the following
prerequisites satisfied:
  * Emscripten SDK
  * Rust
  * OpenSSL (and others)
  * gwasm-runner

### a. Emscripten SDK

Head over to [Emscripten SDK installation page] and follow the instructions there to
get the latest Emscripten SDK installed on your system. **NB** Remember to have `emcc` in your
`PATH` when following the tutorial!

[Emscripten SDK installation page]: https://emscripten.org/docs/getting_started/downloads.html

### b. Rust

#### Getting rustup

We strongly recommend using [rustup] to manage your Rust installations on your favourite OS.
Head over to [rustup] to get the latest copy of `rustup` up and running.

[rustup]: https://rustup.rs/

#### Installing wasm32 target

After you've downloaded and installed `rustup` (make sure that you've added it to your `PATH`!),
you'll need to install the required targets for cross-compilation. You can do this from the
command line/terminal as follows

```
rustup target add wasm32-unknown-emscripten
```

This will install `wasm32-unknown-emscripten` target which is required in order to be able
to cross-compile your Rust apps to gWasm.

### c. OpenSSL and other dependencies

You'll need OpenSSL installed on your system. Note that depending on the OS in question,
you may need some additional packages installed which will be listed in their respective
OS sections below.

#### Ubuntu

FIXME verify this!

On Ubuntu you'll need:
* `libssl-dev`
* `libfreetype6-dev`

You can install both using `apt-get` in your favourite terminal emulator:

```
$ sudo apt-get install libssl-dev libfreetype6-dev
```

#### MacOS

FIXME verify this!

On MacOS you'll need:
* OpenSSL

We strongly recommend using [homebrew] to install these on your Mac:

```
brew install openssl
```

[homebrew]: https://brew.sh/

#### Windows 10

TODO

On Windows 10 you'll need:
* OpenSSL


### d. gwasm-runner

In order to get the latest stable version of [gwasm-runner], we strongly recommend you
download a precompiled binary from the [gwasm-runner releases page].

[gwasm-runner]: https://github.com/golemfactory/gwasm-runner
[gwasm-runner releases page]: https://github.com/golemfactory/gwasm-runner/releases

## 2. Hello World! example

Before tackling some more interesting problems with gWasm, let's first get acquainted with
the `gwasm-api`, the API which we'll use to interface our apps with gWasm. Essentially
speaking, if you can tailor your app to this API, you can run it on gWasm using our
`gwasm-runner` tool! So, without further gilding the lily, let's crack on!

The best way to present an API is by way of example. For the "hello world!" example,
we'll try something really simple. Namely, we will try and sum integers in the range
of `1..100` inclusive, but we will split the task into `10` subtasks, or gWasm tasks.
So how do we do this? We proceed in stages which we'll describe below in more detail:
  1. we split the input array of `100` integers into `10` subarrays such that `[1,...,10]`,
     `[11,...,20]`, `...`, `[91,...,100]`
  2. for each subarray, we calculate the sum of elements; for instance, `sum([1,...,10]) = 55`
  3. finally, we combine all intermediate sums into one final sum, our final value

### i. The gWasm runner API

Before we dig in, please note that you can see the fully assembled example in
[Final result](#final-result). Firstly, just for convenience, let's introduce two
helper "types" (or type aliases in Rust's terminology)

```rust
type Task = Vec<u64>;
type TaskResult = u64;
```

Here, as you've probably already guessed, `Task = Vec<u64>` represents the gWasm task, so
a subarray of integers we will sum to generate the `TaskResult`, i.e., a `u64` value. 

#### Split

`split` function is responsible for splitting the input problem into subproblems, or gWasm
tasks. It's signature can be summarised as follows:

```rust
fn split(ctx: &mut dyn SplitContext) -> Vec<(Task,)>;
```

Firstly, note that, as expected, `split` is required to generate a vector of tasks.
There is one technicality here we need to get our heads round. The API is constructed
in such a way that `split` returns a `Vec` of tuples. Hence, if we have only one
return value as is in this case, we need to wrap it up in a one-element tuple, so
`Vec<(Task,)>`. Furthermore, you've also probably noticed that `split` accepts
a context argument, `SplitContext`. Within this argument, you can interface with
the invoker of your gWasm app with `gwasm-runner` and receive and parse any
passed in command line arguments. We will not dig deeper into this in this tutorial,
but for those interested, feel free to browse our **FIXME [gwasm-api docs]**.

Now, back to our example. Our implementation of `split` needs to generate a vector
of `10` `Task`s. Let's do this then!

```rust
fn split(_ctx: &mut dyn SplitContext) -> Vec<(Task,)> {
    const NUM_SUBTASKS: usize = 10;              // number of gWasm tasks we'll generate
    let arr: Vec<u64> = (1..=100).collect();     // this is our input vector of 100 integers
    let mut tasks: Vec<(Task,)> = Vec::new();    // note the one-element tuple
    for chunk in arr.chunks(NUM_SUBTASKS) {      // split the input into chunks, 10 integers each
        let task: Task = chunk.to_vec();         // convert chunk into Task
        tasks.push((task,));                     // save each task
    }
    tasks
}
```

#### Exec

Having generated gWasm tasks, we now need to provide a method to generate a sum
of each task's elements. The logic that performs this action is represented by
an `exec` function of our API, and it's signature can be summarised as follows:

```rust
fn exec(task: Task) -> (TaskResult,);
```

Just like in `split`'s case, `exec` is subject to the same technicality. That is,
the API is constructed in such a way that `exec` returns a tuple. Hence, if we have
only one return value as is in this case, we need to wrap it up in a one-element tuple.

`exec` function is actually where all the Golem magic happens. Every `Task` passed
into the `exec` function is distributed over GU cluster (when `gwasm-runner`
is used with the GU as the backend), or over Brass network (when `gwasm-runner` is
used with the Brass as the backend). More on that later.

All that's left now is to fill in `exec` with the summing logic, so let's do just that!

```rust
fn exec(task: Task) -> (TaskResult,) {
    let task_result: u64 = task.into_iter().sum(); // this is the sum of our sub-problem
    (task_result,)                                 // note the one-element tuple like for `split`
}
```

#### Merge

Finally, we need to merge the intermediate sums into the final sum, and hence, the
solution to our problem. This is done in the `merge` function:

```rust
fn merge(args: &Vec<String>, results: Vec<((Task,), (TaskResult,))>);
```

`merge` function takes two arguments: `args` vector of `String`s, and `results` vector
of input `Task`s as well as the generated `TaskResult`s. You can think of `args` as 
the owned (for consumption) version of `SplitContext` you saw in `split` function.
We will not dig deeper into the purpose of `args` at this time, and we refer the interested
Reader to our [gwasm-api docs]. `results` vector is more interesting for us at this stage.
Its structure is as follows: for each generated `Task` in the [Split](#split) step,
we have a matching generated `TaskResult` in the [Merge](#merge) step.

Armed with this knowledge, we can finish our app with the `merge` logic, so let's do it!

```rust
fn merge(_args: &Vec<String>, results: Vec<((Task,), (TaskResult,))>) {
    let task_results: Vec<TaskResult> = results.into_iter().map(|(_, (result,))| result).collect(); // extract intermediate sums
    let final_sum: u64 = task_results.into_iter().sum();                                            // merge sums into final sum
    let expected: u64 = (1..=100).sum();                                                            // calculate the sum directly
    assert_eq!(final_sum, expected, "the sums should be equal")                                     // check that both solutions match
}
```

#### Final result

Finally, we can put all of this together into one final `main` function:

```rust
fn main() {
    dispatcher::run(split, exec, merge).unwrap()
}
```

Here, `dispatcher` is part of our `gwasm-api` is essentially speaking, it 
ties all 3 stages together.

Putting everything together, we get the following `main.rs` file for our
"hello world!" app:

```rust
// main.rs
use gwasm_api::{dispatcher, SplitContext};                                                                                                              
                                                                                                                        
fn main() {                                                                                                             
    dispatcher::run(split, exec, merge).unwrap()                                                                        
}                                                                                                                       
                                                                                                                        
type Task = Vec<u64>;                                                                                                   
type TaskResult = u64;                                                                                                  
                                                                                                                        
fn split(_ctx: &mut dyn SplitContext) -> Vec<(Task,)> {                                                                 
    const NUM_SUBTASKS: usize = 10;                                                                                     
    let arr: Vec<u64> = (1..=100).collect();                                                                            
    let mut tasks: Vec<(Task,)> = Vec::new();                                                                           
    for chunk in arr.chunks(NUM_SUBTASKS) {                                                                             
        let task: Task = chunk.to_vec();                                                                                
        tasks.push((task,));                                                                                            
    }                                                                                                                   
    tasks                                                                                                               
}                                                                                                                       
                                                                                                                        
fn exec(task: Task) -> (TaskResult,) {                                                                                  
    let task_result: u64 = task.into_iter().sum();                                                                      
    (task_result,)                                                                                                      
}                                                                                                                       
                                                                                                                        
fn merge(_args: &Vec<String>, results: Vec<((Task,), (TaskResult,))>) {                                                 
    let task_results: Vec<TaskResult> = results.into_iter().map(|(_, (result,))| result).collect();                     
    let final_sum: u64 = task_results.into_iter().sum();                                                                
    let expected: u64 = (1..=100).sum();                                                                                
    assert_eq!(final_sum, expected, "the sums should be equal")                                                         
}
```

Don't forget to add `gwasm-api` as a dependency in your `Cargo.toml`:

```rust
// Cargo.toml
[dependencies]
gwasm-api = { git="https://github.com/golemfactory/gwasm-runner.git" }
```

All of the code presented in this tutorial is available for you prepackaged as a crate
inside your Docker environment in `/root/hello/` folder. You can browse the `main.rs` 
using Vi editor as follows:

```
vi /root/hello/src/main.rs
```

Alternatively, if you want to clone the source, you can do so by cloning [hello-gwasm-runner]
on Github. There, you'll find two branches: [master] and [workshop]. The `workshop` branch contains
the version of the program described in this tutorial, whereas `master` showcases an alternative
view at the API. Feel free to browse especially if you want to explore slightly more advanced
Rust usage ;-)

[hello-gwasm-runner]: https://github.com/golemfactory/hello-gwasm-runner
[master]: https://github.com/golemfactory/hello-gwasm-runner/tree/master
[workshop]: https://github.com/golemfactory/hello-gwasm-runner/tree/workshop

### ii. Build

Let's try and build our "hello world!" app. Regardless of whether you'll be doing it inside Docker
or on your OS, in the terminal run:

```
cargo build --release
```

You can find the built artifacts in `target/wasm32-unknown-emscripten/release` directory.

### iii. Run!

In order to execute our cool "hello world!" app, we'll use `gwasm-runner`, and we'll run it
using two backends: locally (all using your own machine), and on the GU cluster.

#### Run locally

```
gwasm-runner target/wasm32-unknown-emscripten/release/hello.wasm
```

#### Run on the GU cluster

**FIXME**

```
gwasm-runner ???
```

## 3. Mandelbrot example
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
