# Fractal renderer

This was a fun but brief project I worked on.

Most of the functionallity is implemented.
If you have any ideas regarding colouring and general performance, please reach out.
I'd also ultimately like to render this on the GPU.

Rendering the Mandelbrot set at 1,000,000x zoom.
![mandelbrot at 1000000x zoom](screenshots/mandelbrot-1000000x.avif)
It took my shitty laptop ~1 second to render the 3000x3000 image above. And that's on the CPU!

# Installation

You have to have NASM installed to build the image compression library.
In the future, I'll make the feature optional and enable you to use other image formats instead.

## Using the GPU feature

This renders the fractals on the GPU.

```bash
$ rustup install nightly-2022-01-13
$ rustup update
$ rustup component add --toolchain nightly-2022-01-13 rust-src rustc-dev llvm-tools-preview
```

Run using the folowing

```bash
$ cargo +nighly-2022-01-13 r --feaures gpu
```

# Examples

To give arguments to this binary when using `cargo run --release`, add them after two hyphens: `cargo r --release -- <arguments>`.

Look at [the examples MD doc](examples.md).

# Contribution

This project is dual-licensed under Apache 2.0 or MIT.
All contributions are assumed to also be.
