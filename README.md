# nextgen

A static site generator in Rust.

Built to see how much code is really needed to replicate [my website](https://f5n.org) that is built with hugo.

The result are 333 lines of code (282 NCLOC) and using 6 crates.

[Here's a detailed writeup](https://f5n.org/blog/2020/a-static-site-generator/) about the development.


## How to build

Have a somewhat current version of stable Rust, 1.38.0 works fine.

```
# get a theme
git clone https://github.com/winks/nextgen-themes
ln -s nextgen-themes/f5n.org theme
```

```
# build and run
cargo build --release

mkdir public
./target/release/nextgen
```


## License

ISC
