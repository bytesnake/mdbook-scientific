# Scientific mdbook plugin
This plugin adds functionality to `mdbook` for scientific application. It allows the writer to generate named LaTeX :
````
```latex, <name>, <subtitle>
...
```
````

and then cross-reference with `` `ref:fig:<name>` ``.

## Install

Compile this crate and add the `mdbook-scientific` to your search path. Also [bib2xhtml](https://github.com/dspinellis/bib2xhtml) is required if you want to generate a bibliography. Then add the following to your `book.toml`:
```
[preprocessor.scientific]
renderer = ["html"]

bibliography = "literature.bib"
bib2xhtml = "/home/lorenz/Documents/tmp/bib2xhtml/"

assets = "src/"

[output.html]
additional-css = ["src/scientific.css"]
```

## Should I use this
Nope, it's still in its infancy. Please don't use it yet 

