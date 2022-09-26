# Scientific mdbook plugin

This plugin adds functionality to `mdbook` for scientific application. 

It allows the writer to generate named `LaTeX`, named `Gnuplots` and supports `bibtex` files. Further cross-referencing in text to equation, figures or literature is possible. A sample output can be seen [here](https://bytesnake.github.io/mdbook_example/).

## Install

Compile this crate and add the `mdbook-scientific` to your search path. Also [bib2xhtml](https://github.com/dspinellis/bib2xhtml) is required if you want to generate a bibliography. Then add the following to your `book.toml`:

```toml
[preprocessor.scientific]
renderer = ["html"]

bibliography = "literature.bib"
bib2xhtml = "/home/lorenz/Documents/tmp/bib2xhtml/"

assets = "src/"

[output.html]
additional-css = ["src/scientific.css"]
```

## Prerequisites

* Formulae and general latex rendering `latex` and `dvisvgm`
* Graphs require `gnuplot`

## Syntax

For block equation rendering use the following syntax

```md
$$equation, <name>
...
$$
```

the `equation` identifier is only needed if you want to name the equation block. You can cross-reference it then with `$ref:equ:<name>$` in the whole `mdbook`.

The same syntax is working with `latex` and `gnuplot` figures, both are requiring a subtitle for the plot. Further a `gnuplotonly` figure only uses Gnuplot to render the file to SVG.

Example for gnuplot rendering

```md
$$gnuplot, <name>, <subtitle>
...
$$
```

and then cross-reference with `$ref:fig:<name>$`.

If block is empty, then the preprocessor looks into the `assets` path specified in the configuration. So for a block `$$latex, legendrepoly, Legendre Polynomials$$` it looks for the file `src/legendrepoly.tex`.

The BibTeX file referenced in the configuration file is added as a additional chapter and citations can be generated with `$ref:bib:<name>$`.

## Stability / Viability

Proof of concept, with the following outstanding urgent todos for practical viability:

* [x] handle `$` signs in code blocks
* [ ] migrate to a full markdown parser rather than impl heuristics
* [ ] remove dependencies on host binaries as much as possible
