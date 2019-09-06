# gwasm-runner
Command line tool for running gWasm compatible apps locally, via Golem Unlimited or via Brass Golem.

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

```
sudo apt-get install clang-6.0 autoconf2.13 yasm
cargo build --release
```
