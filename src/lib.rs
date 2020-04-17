mod error;
mod fragments;
mod preprocess;

use std::collections::HashMap;
use std::path::Path;
use std::fs;

use mdbook::book::{Book, BookItem};
use mdbook::errors::Error;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};

use preprocess::{replace_blocks, replace_inline_blocks};

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

            // track which fragments we use to copy them into the assets folder
            let mut used_fragments = Vec::new();
            // track which references are created
            let mut references = HashMap::new();
            // if there occurs an error skip everything and return the error
            let mut error = None;

            book.for_each_mut(|item| {
                if error.is_some() {
                    return;
                }

                if let BookItem::Chapter(ref mut ch) = item {
                    let head_number = ch.number.as_ref().map(|x| format!("{}", x)).unwrap_or("".into());

                    match replace_blocks(&fragment_path, &ch.content, &head_number, &mut used_fragments, &mut references) {
                        Ok(x) => ch.content = x,
                        Err(err) => error = Some(format!("Error in chapter {} {:?}", head_number, err))
                    }
                }
            });

            book.for_each_mut(|item| {
                if error.is_some() {
                    return;
                }

                if let BookItem::Chapter(ref mut ch) = item {
                    let head_number = ch.number.as_ref().map(|x| format!("{}", x)).unwrap_or("".into());

                    match replace_inline_blocks(&fragment_path, &ch.content, &references, &mut used_fragments) {
                        Ok(x) => ch.content = x,
                        Err(err) => error = Some(format!("Error in chapter {}: {:?}", head_number, err))
                    }
                }
            });

            if let Some(err) = error {
                return Err(err.into());
            }

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

