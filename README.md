# nextgen

A static site generator in Rust.

Inspired by hugo, incentivised by hugo's too many features.


## How to build

Have a somewhat current version of stable Rust, 1.61.0 works fine.

```
# get a theme
git clone https://github.com/winks/nextgen-themes
ln -s nextgen-themes/f5n.org theme
```

```
# build and run
cargo build --release

# build the example site
cd example
./target/release/nextgen

# or run your own
cp ../nextgen.toml.default nextgen.toml
mkdir {content,public}
cp -r example/theme .
./target/release/nextgen
```

## History

Built to see how much code is really needed to replicate [my website](https://f5n.org) that is built with hugo.

The result originally were 333 lines of code (282 NCLOC) and 6 crates, but with a few features missing.

[Here's a detailed writeup](https://f5n.org/blog/2020/a-static-site-generator/) about the development.

## License

ISC
