use fs_err as fs;
use itertools::Itertools;
use std::collections::HashMap;
use std::path::Path;

use crate::error::{Error, Result};
use crate::fragments;

const BLOCK_DELIM: &str = "$$";
const INLINE_BLOCK_DELIM: &str = "$";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LiCo {
    /// Base 1 line number
    pub lineno: usize,
    /// Base 1 column number
    pub column: usize,
}

pub struct Content<'a> {
    pub s: &'a str,
    pub start: LiCo,
    pub end: LiCo,
}

impl<'a> AsRef<str> for Content<'a> {
    fn as_ref(&self) -> &str {
        self.s
    }
}

impl<'a> std::ops::Deref for Content<'a> {
    type Target = &'a str;
    fn deref(&self) -> &Self::Target {
        &self.s
    }
}

/// Takes a file residing at path, and uses it to produce
/// includable rendered equations.
pub fn replace_blocks(
    fragment_path: &Path,
    asset_path: &Path,
    source: &str,
    head_num: &str,
    used_fragments: &mut Vec<String>,
    references: &mut HashMap<String, String>,
) -> Result<String> {
    let mut content = String::new();

    let mut start_loco: Option<(LiCo, String)> = None;

    let mut figures_counter = 0;
    let mut equations_counter = 0;

    let mut add_object = move |file: String, refer: &str, title: Option<&str>| -> String {
        used_fragments.push(file.clone());

        if let Some(title) = title {
            figures_counter += 1;
            references.insert(
                refer.to_string(),
                format!("Figure {}{}", head_num, figures_counter),
            );

            format!(
                r#"<figure id="{}" class="figure"><object data="assets/{}" type="image/svg+xml"/></object><figcaption>Figure {}{} {}</figcaption></figure>"#,
                refer, file, head_num, figures_counter, title
            )
        } else if !refer.is_empty() {
            equations_counter += 1;
            references.insert(
                refer.to_string(),
                format!("{}{}", head_num, equations_counter),
            );
            format!(
                r#"<div id="{}" class="equation"><div class="equation_inner"><object data="assets/{}" type="image/svg+xml"></object></div><span>({}{})</span></div>\n"#,
                refer, file, head_num, equations_counter
            )
        } else {
            format!(
                r#"<div class="equation"><div class="equation_inner"><object data="assets/{}" type="image/svg+xml"></object></div></div>\n"#,
                file
            )
        }
    };

    fs::create_dir_all(&fragment_path)?;

    let mut acc = Vec::<String>::with_capacity(100);

    for (lineno, line) in source.lines().enumerate() {
        let leading_white = line.chars().take_while(|c| c.is_whitespace()).count();
        // let _trailing_white = line.chars().rev().take_while(|c| c.is_whitespace()).count();

        let line = line.trim();
        let loco = LiCo {
            lineno: lineno + 1,
            column: leading_white + 1,
        };

        // look for a block
        if !line.starts_with(BLOCK_DELIM) {
            if start_loco.is_some() {
                content.push_str(line);
                content.push('\n');
                continue;
            } else {
                acc.push(line.to_owned());
                continue;
            }
        } else if line.ends_with(BLOCK_DELIM) && line.len() > 3 {
            // line starts and end with BLOCK_DELIM
            // set content to empty
            start_loco = Some((loco, line.to_string()));
            content = "".into();
        }

        if let Some((start_loco, param)) = start_loco.take() {
            let elms = param
                .splitn(3, ",")
                .map(|x| x.trim())
                .map(|x| x.replace(BLOCK_DELIM, ""))
                .collect::<Vec<_>>();

            let elms = elms.iter().map(|x| x.as_str()).collect::<Vec<_>>();

            // if there is no content, try to load it from file
            if content.is_empty() {
                let path = asset_path.join(elms[1]).with_extension("tex");
                if path.exists() {
                    content = fs::read_to_string(path)?;
                } else {
                    eprintln!("Block empty, but file `{}` was not found!", elms[1]);
                    continue;
                }
            }

            {
                let content = Content {
                    s: content.as_str(),
                    start: start_loco,
                    end: loco,
                };

                let generated_out = match &elms[..] {
                    ["latex", refer, title] => fragments::parse_latex(fragment_path, &content)
                        .map(|file| add_object(file, refer, Some(title))),
                    ["gnuplot", refer, title] => fragments::parse_gnuplot(fragment_path, &content)
                        .map(|file| add_object(file, refer, Some(title))),
                    ["gnuplotonly", refer, title] => {
                        fragments::parse_gnuplot_only(fragment_path, &content)
                            .map(|file| add_object(file, refer, Some(title)))
                    }

                    ["equation", refer] | ["equ", refer] => {
                        fragments::parse_equation(fragment_path, &content, 1.6)
                            .map(|file| add_object(file, refer, None))
                    }

                    ["equation"] | ["equ"] | _ => {
                        fragments::parse_equation(fragment_path, &content, 1.6)
                            .map(|file| add_object(file, "", None))
                    }
                }?;
                acc.push(generated_out)
            }
            content = String::new();
        } else {
            start_loco = Some((loco, line.to_string()));
            continue;
        }
    }
    Ok(acc.join("\n"))
}

pub fn replace_inline_blocks(
    fragment_path: &Path,
    source: &str,
    references: &HashMap<String, String>,
    used_fragments: &mut Vec<String>,
) -> Result<String> {
    let mut is_code_block = false;
    source
        .lines()
        .enumerate()
        .map(|(lineno, linecontent)| {
            // FIXME use a proper markdown/commonmark parser, it's unfixable this
            // way i.e pre start and end in one line or multiple..
            if linecontent.starts_with("```") {
                is_code_block = !is_code_block;
                return Ok(linecontent.to_owned());
            }

            if linecontent.starts_with("<pre") {
                is_code_block = true;
                return Ok(linecontent.to_owned());
            }

            if linecontent.starts_with("</pre>") {
                is_code_block = false;
                return Ok(linecontent.to_owned());
            }

            let mut is_intra_inline_code = false;
            // use to collect ranges
            let mut v = vec![-1_isize];
            v.extend(linecontent.chars().enumerate().filter_map(|(i, c)| {
                match c {
                    '$' if !is_intra_inline_code => {
                        return Some(i as isize);
                    }
                    '`' => {
                        is_intra_inline_code = !is_intra_inline_code;
                    }
                    _ => {}
                }
                None
            }
        ));
            


            v.into_iter()
                .tuple_windows()
                .enumerate()
                .map(|(i, (start, end))| {
                    let start = std::cmp::min(start + 1, end) as usize;
                    let end = end as usize;

                    let elm = &linecontent[start..end];

            if i % 2 == 0 {
                // no within, so just return a string
                return Ok(elm.to_owned());
            }

            let content = Content {
                s: elm,
                start: LiCo {
                    lineno,
                    column: start,
                },
                end: LiCo {
                    lineno,
                    column: end,
                },
            };

            let generated_out = if elm.starts_with("ref:") {
                let elms = elm.split(":").skip(1).collect::<Vec<&str>>();

                // we expect a type and reference name
                if elms.len() != 2 {
                    // if not just return as text again
                    return Ok(elm.to_string());
                }

                match &elms[..] {
                    ["fig", refere] => {
                        references.get::<str>(refere)
                            .ok_or(Error::InvalidReference(format!(r#"could not find reference to `{}` in line {}"#, elms[1], lineno)))
                            .map(|x| format!(r#"<a class="fig_ref" href='#{}'>{}</a>"#, elms[1], x))
                    },
                    ["bib", refere] => {
                        references.get::<str>(refere)
                            .ok_or(Error::InvalidReference(format!("could not find reference to `{}` in line {}", elms[1], lineno)))
                            .map(|x| format!(r#"<a class="bib_ref" href='bibliography.html#{}'>{}</a>"#, elms[1], x))
                    },
                    ["equ", refere] => {
                        references.get::<str>(refere)
                            .ok_or(Error::InvalidReference(format!("could not find reference to `{}` in line {}", elms[1], lineno)))
                            .map(|x| format!(r#"<a class="equ_ref" href='#{}'>Eq. ({})</a>"#, elms[1], x))
                    },
                    [kind, _] => Err(Error::InvalidReference(format!("unknown reference type of `{}` in line {}", kind, lineno))),
                    _ =>         Err(Error::InvalidReference(format!("reference has wrong number of arguments `{}` in line {}", elms.len(), lineno)))

                }
            } else {
                fragments::parse_equation(fragment_path, &content, 1.3)
                    .map(|filename| {
                        let res = format!(r#"<object class="equation_inline" data="assets/{}" type="image/svg+xml"></object>"#, filename);
                        used_fragments.push(filename);

                        res
                    })
            };

            generated_out
        })
        .collect::<Result<Vec<String>>>()
        .map(|x| x.join("\n"))
    })
    .collect::<Result<Vec<_>>>()
    .map(|x| x.join("\n"))
}
