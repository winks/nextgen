# nextgen

A static site generator in Rust.

Inspired by hugo, incentivised by hugo's too many features.


## How to build

Have a somewhat current version of stable Rust, 1.72.0 works fine.


```
# build and run
cargo build --release

# build the example site
cd example

# optionally, get a different blueprint:
mv blueprints blueprints.example
git clone https://github.com/winks/nextgen-blueprints blueprints

../target/release/nextgen

# or run your own
mkdir newsite
cd newsite
cp ../nextgen.toml.default ./nextgen.toml
mkdir content
cp -r ../example/blueprints .
cp ../example/content/_index.md content/
../target/release/nextgen
```

## Gotchas

  * blueprints folder must not be a symlink
  * no cli args
  * panic on error


## History

Built to see how much code is really needed to replicate [my website](https://f5n.org) that is built with hugo.

The result originally were 333 lines of code (282 NCLOC) and 6 crates, but with a few features missing.

[Here's a detailed writeup](https://f5n.org/blog/2020/a-static-site-generator/) about the development.

## License

ISC
