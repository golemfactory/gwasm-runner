# gwasm-runner [![Build Status](https://github.com/golemfactory/gwasm-runner/workflows/Continuous%20integration/badge.svg)](https://github.com/golemfactory/gwasm-runner/actions?workflow=Continuous%20integration)


Command line tool for running gWasm compatible apps locally, via Golem Unlimited or via Brass Golem.

It introduces simplistic API that resembles map-reduce paradigm. 

This API enables developers to easily implement simple applications and run them on top of the Golem Unlimited and also on [Brass Golem 0.21 and later](https://blog.golemproject.net/brass-golem-beta-0-21-0-hello-mainnet-gwasm/).

## building on macos

You need to have C compiler. Try
```
xcode-select --install
```

Use [Homebrew](https://brew.sh/#install) to install Python 2, AutoConf 2.13 and Yasm:
```bash
brew install python@2 autoconf@2.13 yasm

# remove newer version of autoconf
brew unlink autoconf
# and use 2.13 instead
ac=`which autoconf213` && sudo ln -s "$ac" "${ac%213}"
```

Now you can build 
```
cargo build --release
```

## building on Linux

You need to have python2 alias to python2.7
```
sudo apt-get install clang-6.0 autoconf2.13 yasm
cargo build --release
```

## Running a WASM binary
The first step here is obtaining a WASM binary compatible with the runner. This means that the binary must conform to the runner's split-execute-merge API.
A list of compatible binaries can be found in the [wasm-store](https://github.com/golemfactory/wasm-store) repository. Of course, you can also create your own program.
As our example we're going to use [mandelbrot](https://github.com/golemfactory/mandelbrot), a Mandelbrot fractal visualiser.

### Running on the Golem network
Using gwasm-runner, you can execute the WASM binary in the Golem network, taking advantage of parallelism by splitting the work between multiple providers.

To create a task you will need to have an instance of Golem (version 0.21+) running locally on your machine. This instance will act as a requestor within the Golem network.
Assuming that we want to run the mandelbrot example, issue the below command:

```
./gwasm-runner /path/to/mandelbrot.wasm --backend=Brass -- 1000 1000 4
```

Let's take a closer look at that command:
- The first argument to the runner is the path to the WASM binary. Please note that the runner expects the corresponding `.js` file to be present in the same directory as the WASM binary.
- `Brass` stands for Brass Golem, the name for the current iteration of the Golem project.
- The parameters after `--` are passed directly to the WASM program, therefore they are app-specific. In the case of the mandelbrot example, the first two numbers (`1000 1000`) are the width and height of the output image. The last number (`4`) is the subtask count, which determines the number subtasks we want to split our work into.

When creating a task in the Golem network, there are some more parameters which can be defined. For example, we want to be able to specify how much we're willing to pay for the computation or what the timeout for a task should be. These Golem-specific parameters can be defined in a configuration file which will be used by the runner. By default, these values are the following:

```
{
    "data_dir": "/home/user/.local/share/golem/default",
    "address": "127.0.0.1:61000",
    "bid": 1.0,
    "name": "gwasm-task",
    "net": "testnet",
    "subtask_timeout": "00:10:00",
    "task_timeout": "00:30:00"
}
```

To change the default values (e.g. when the datadir for your local Golem instance is located somewhere else) a JSON configuration file needs to be created. Under Linux, this file should be placed under: `~/.config/g-wasm-runner/brass/config.json`. You can copy the above JSON object, modify the fields' values and put it into that file. The runner will print its currently used configuration upon start-up.
