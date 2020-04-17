# Scientific mdbook plugin
This plugin adds functionality to `mdbook` for scientific application. It allows the writer to generate named LaTeX plots with
```
$$latex, <name>, <subtitle>
...
$$
```

and then cross-reference with `$ref:fig:<name>$`.

## What remains to implement
 * [ ] equation rendering with references like `$ref:equ:<name>$`
 * [ ] BibTeX bibliography rendering with `bib2xhtml` and references like `$ref:bib:<name>$`
 * [ ] support for gnuplot and gnuplot-latex combination

## Should I use this
Nope, it's still in its infancy. Please don't use it yet 

