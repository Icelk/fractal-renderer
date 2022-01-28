`--open` to open the image after completion.

ALWAYS run with `--release`!

# Julia

- `-a julia --julia-real -0.8 --julia-imaginary 0.156 --open -i 2000 -s 0.6 -e 30 2000 1000`
- `-a julia --julia-real -0.7269 --julia-imaginary 0.1889 --open -i 1000 3000 1500`
- `-a julia --julia-real -0.70176 --julia-imaginary 0.3842 --open -i 400 -e 25 3000 1500`
- `-a julia --julia-real 0.285 --julia-imaginary 0.01 --open -i 100 -e 10 2500 3000`
- `-a julia --julia-real -0.2256 --julia-imaginary 0.65 --open -i 500 -e 12 -x 0.29449 -y -0.40460 2000 1000`
- `-a julia --julia-real 0.36105 --julia-imaginary 0.35977 -e 6 -i 500`

# Mandelbrot

- Classic: `-d 3000 2000`
- Golden: `<no arguments>`
- Golden fringe: `-i 400`

All of [the Wikipedia zoom gallery](https://en.wikipedia.org/wiki/Mandelbrot_set#Image_gallery_of_a_zoom_sequence):

> The scale of this program needs to be ~0.4 of those described in the article.
> Why the iterators are so different is because they contribute to the brightness of the "background".

- `-s 400 -x -0.7435669 -y 0.1314023 --open -i 5000 -e 10`
- `-s 2000 -x -0.74364990 -y 0.13188204 --open -i 800`
- `-s 12000 -x -0.74364085 -y 0.13182733 --open -i 5000 -e 1'`
- `-s 100000 -x -.743643135 -y  .131825963 --open -i 2000 -d -e 3`
- `-s 500000 -x -.7436447860 -y  .1318252536 --open -i 4000 -d -e 5 4000 2000`

# Fern

- Like exactly the one from [Wikipedia](https://en.wikipedia.org/wiki/Barnsley_fern#/media/File:Barnsley_fern_1024x1024.png) `-a fern 1000 1000`
