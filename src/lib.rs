mod error;
mod fragments;

use std::collections::HashMap;
use std::path::Path;
use std::fs;

use mdbook::book::{Book, BookItem};
use mdbook::errors::Error;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};

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
            let fragment_path = cfg
                .get("fragment_path")
                .map(|x| x.as_str().unwrap())
                .unwrap_or("fragments/");
            let fragment_path = Path::new(fragment_path).canonicalize().unwrap();

            let mut used_fragments = Vec::new();
            let mut references = HashMap::new();

            book.for_each_mut(|item| match item {
                BookItem::Chapter(ch) => {
                    match replace_blocks(&fragment_path, &ch.content, ch.number.as_ref().map(|x| format!("{}", x)).unwrap_or("1".into()), &mut used_fragments, &mut references) {
                        Ok(x) => ch.content = x,
                        Err(err) => panic!("Could not replace blocks: {:?}", err)
                    }
                }
                _ => {}
            });

            book.for_each_mut(|item| match item {
                BookItem::Chapter(ch) => {
                    ch.content = replace_inline_blocks(&ch.content, &references);
                },
                _ => {}
            });

            // copy all used fragments to `assets`
            let dest = ctx.root.join("src").join("assets");
            if !dest.exists() {
                fs::create_dir_all(&dest).unwrap();
            }

            for fragment in used_fragments {
                fs::copy(fragment_path.join(&fragment), dest.join(&fragment)).unwrap();
            }

            Ok(book)
        } else {
            Err("Key section not found!".into())
        }
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer != "not-supported"
    }
}

fn replace_blocks(fragment_path: &Path, source: &str, head_num: String, used_fragments: &mut Vec<String>, references: &mut HashMap<String, String>) -> error::Result<String> {
    let mut content = String::new();
    let mut start_line: Option<String> = None;
    let mut figures_counter = 0;

    Ok(source.split("\n")
    .filter_map(|line| {
        if !line.starts_with("$$") {
            if start_line.is_some() {
                content.push_str(line);
                content.push('\n');
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
                Err(err) => panic!("{:?}", err)
            }
        } else {
            start_line = Some(line.to_string());
            None
        }
    })
    .collect::<String>())
}

fn replace_inline_blocks(source: &str, references: &HashMap<String, String>) -> String {
    source.split("\n").map(|line| {
        if line.matches("$").count() % 2 != 0 {
            panic!("Uneven number of $ in line");
        }

        line.split("$").enumerate().map(|(i, mut elm)| {
            if i % 2 == 0 {
                return elm.to_string();
            }

            let generated_out = if elm.starts_with("ref") {
                let elms = elm.split(":").skip(1).collect::<Vec<&str>>();

                match &elms[..] {
                    ["fig", refere] => {
                        references.get::<str>(refere)
                            .map(|x| format!("<a class=\"fig_ref\" href='#{}'>{}</a>", elms[1], x))
                            .unwrap()
                    },
                    _ => panic!("Blub")
                }
            } else {
                panic!("");
            };

            /*match generated_out {
                Ok(generated_out) => generated_out,
                Err(_) => panic!("cds")
            }*/
            generated_out
        }).collect::<String>()
    }).collect::<String>()
}
