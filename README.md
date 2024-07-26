# WordRoute

WordRoute as a website for a word search game heavily inspired by
[Squardle](https://squaredle.app/). The main difference is that it’s
open source, uses the [Shavian alphabet](https://shavian.info) and has
hexagons instead of squares.

If you just want to play the game you can find it at
[wordroute.busydoingnothing.co.uk](https://wordroute.busydoingnothing.co.uk).

## Building the website

Make sure you have a [Rust compiler](https://rustup.rs/) and
[wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
installed. Then you can type:

```bash
wasm-pack build --target=web
./create-dist.sh
```

Then the files needed for the website will be ready in a directory
called `dist`. The game is effectively a static website so you can
just copy them somewhere where a web server can see them and start
using it. Note that most browsers won’t load WebAssembly from a
`file:///` URL for some reason, so you can’t run the game locally
without a web server.

## Adding puzzles

All of the puzzles are made by hand in order to ensure the words
aren’t too weird and the puzzle is fun to play. You can make your own
puzzles by just drawing a grid in a text editor and then running it
through the build tool. The tool ignores spaces in the grid so you can
lay it out to make a visible honeycomb pattern. You can use full stops
(‘.’) to leave a gap in the puzzle. For example, to make the shape of
the first puzzle, you can use this grid:

```
     . 𐑱 𐑖 𐑩
      𐑼 𐑦 𐑤 𐑯
     𐑦 𐑑 𐑟 𐑮 𐑴
      𐑙 𐑯 𐑨 𐑑
     . 𐑒 𐑼 𐑟
```

### Building the dictionary

In order to run the puzzle generation tool, you first need a
dictionary file. This is extracted from the ReadLex JSON. You can
generate this with the following commands:

```bash
git clone https://github.com/Shavian-info/readlex.git
cargo run \
      --release \
      --bin=extract-dictionary \
      -- \
      --dictionary dictionary.txt \
      --bonus-words bonus-words.txt \
      --readlex readlex/readlex.json
```

Now you should have a list of words in `dictionary.txt` and another
list of what the program deems to be bonus words from the ReadLex in
`bonus-words.txt`. In order to use the dictionary you first need to
convert it to a binary format. The tool to do that is in the repo for
[Vaflo](https://vaflo.net). You can use the following commands to do
it:

```bash
git clone https://github.com/bpeel/vaflo
cargo run --manifest-path vaflo/Cargo.toml \
      --release \
      --bin=make-dictionary \
      -- \
      dictionary.bin \
      < dictionary.txt
```

### Visualising the puzzle

Once you have the dictionary file you can run the grid through the
build tool to get a list of words that it finds. Assuming you have
saved your puzzle in a text file called `my-puzzle.txt`, you can run
the tool like this:

```bash
cargo run --release \
      -- \
      --dictionary dictionary.bin \
      --bonus-words bonus-words.txt \
      --human-readable \
      my-puzzle.txt
```

Check that all of the letters in your grid are used at least once by
looking at the counts under each letter. Now you can read through the
word list and decide which words should be bonus words and which
should be excluded. For this example, you can list your new bonus
words in a file called `extra-bonus-words.txt` and the excluded words
in `excluded-words.txt`.

### Generating the puzzle code

Once you are happy with the result you can
generate the final puzzle code with the following command:

```bash
cargo run --release \
      -- \
      --dictionary dictionary.bin \
      --bonus-words bonus-words.txt \
      --bonus-words extra-bonus-words.txt \
      --excluded-words excluded-words.txt \
      my-puzzle.txt
```

This will generate the code for the puzzle as a single line of ASCII
text. If you add this to `puzzles.txt` and then point your browser at
a web server hosting the root directory of the git repo, you can test
your new puzzle in the browser.
