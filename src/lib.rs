mod error;
mod fragments;

use std::path::Path;
use std::collections::HashMap;

use mdbook::book::{Book, BookItem};
use mdbook::errors::Error;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};

use fragments::parse_latex;

pub struct Scientific;

impl Scientific {
    pub fn new() -> Scientific {
        Scientific
    }
}

impl Preprocessor for Scientific {
    fn name(&self) -> &str {
        "scientific"
    }

    fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
        // In testing we want to tell the preprocessor to blow up by setting a
        // particular config value
        if let Some(cfg) = ctx.config.get_preprocessor(self.name()) {
            let fragment_path = cfg.get("fragment_path").map(|x| x.as_str().unwrap()).unwrap_or("fragments/");

            book.for_each_mut(|item| {
                match item {
                    BookItem::Chapter(ch) => { replace_blocks(Path::new(&fragment_path), ch.content.clone()); },
                    _ => {}
                }
            });

            Ok(book)
        } else {
            Err("Key section not found!".into())
        }

    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer != "not-supported"
    }
}

fn replace_blocks(fragment_path: &Path, source: String) -> String {
    let mut content = String::new();
    let mut start_line: Option<String> = None;
    let mut references = HashMap::new();
    let mut used_fragments = Vec::new();
    let mut figures_counter = 0;
    let head_num = 0;

    source.split("\n")
    .filter_map(|line| {
        if !line.starts_with("$$") {
            if start_line.is_some() {
                content.push_str(line);
                content.push_str("\n");
                return None;
            } else {
                return Some(format!("{}\n", line));
            }
        }

        if let Some(ref param) = start_line {
            let mut elms = param.splitn(3, ",").map(|x| x.trim());

            let generated_out = match elms.next() {
                Some("$$latex") => {
                    figures_counter += 1;
                    fragments::parse_latex(fragment_path, elms.map(|x| x.to_string()).collect(), &content)
                        .map(|(file, title, refer)| {
                            used_fragments.push(file.clone());
                            references.insert(refer.clone(), format!("Figure {}.{}", head_num, figures_counter));

                            format!("<figure id=\"{}\" class=\"figure\"><object data=\"/assets/{}\" type=\"image/svg+xml\"/></object><figcaption>Figure {}.{} {}</figcaption></figure>", refer, file, head_num, figures_counter, title)
                        })
                },
                _ => Err(error::Error::InvalidCode("blub".into()))
            };
            content = "".into();
            start_line = None;

            match generated_out {
                Ok(generated_out) => Some(format!("{}\n", generated_out)),
                Err(_) => panic!("Could not generate!")
            }
        } else {
            None
        }
    })
    .collect::<String>()
}
