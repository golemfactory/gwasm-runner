# gwasm-runner
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
