use fs_err as fs;
use itertools::Itertools;
use std::collections::HashMap;
use std::path::Path;

use crate::error::{Error, Result};
use crate::fragments;

const BLOCK_DELIM: &str = "$$";
const INLINE_BLOCK_DELIM: &str = "$";

pub fn replace_blocks(
    fragment_path: &Path,
    asset_path: &Path,
    source: &str,
    head_num: &str,
    used_fragments: &mut Vec<String>,
    references: &mut HashMap<String, String>,
) -> Result<String> {
    let mut content = String::new();
    let mut start_line: Option<String> = None;
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

            format!("<figure id=\"{}\" class=\"figure\"><object data=\"assets/{}\" type=\"image/svg+xml\"/></object><figcaption>Figure {}{} {}</figcaption></figure>",
                refer, file, head_num, figures_counter, title)
        } else if !refer.is_empty() {
            equations_counter += 1;
            references.insert(
                refer.to_string(),
                format!("{}{}", head_num, equations_counter),
            );
            format!("<div id=\"{}\" class=\"equation\"><div class=\"equation_inner\"><object data=\"assets/{}\" type=\"image/svg+xml\"></object></div><span>({}{})</span></div>\n", refer, file, head_num, equations_counter)
        } else {
            format!("<div class=\"equation\"><div class=\"equation_inner\"><object data=\"assets/{}\" type=\"image/svg+xml\"></object></div></div>\n", file)
        }
    };

    let mut acc = Vec::<String>::with_capacity(100);

    for line in source.lines() {
        let line = line.trim();

        if !line.starts_with(BLOCK_DELIM) {
            if start_line.is_some() {
                content.push_str(line);
                content.push('\n');
                continue;
            } else {
                acc.push(line.to_owned());
                continue;
            }
        } else if line.ends_with(BLOCK_DELIM) && line.len() > 3 {
            // line starts and end with BLOCK_DELIM, set content to empty
            start_line = Some(line.to_string());
            content = "".into();
        }

        if let Some(param) = start_line.take() {
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
            content = "".into();

            acc.push(generated_out)
        } else {
            start_line = Some(line.to_string());
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
    source.lines().enumerate().map(|(line_num, line)| {
// FIXME use a proper mardown parser, it's unfixable this way
// i.e pre start and end in one line or multiple..
        if line.starts_with("```") {
            is_code_block = !is_code_block;
            return Ok(line.to_owned());
        }

        if line.starts_with("<pre") {
            is_code_block = true;
            return Ok(line.to_owned())
        }

        if line.starts_with("</pre>") {
            is_code_block = false;
            return Ok(line.to_owned());
        }

        let mut is_intra_inline_code = false;
        // use to collect ranges
        let mut v = vec![-1_isize];
        v.extend(line.chars().enumerate().filter_map(|(i,c)| {
            match c {
                '$' if !is_intra_inline_code => {
                    return Some(i as isize);
                }
                '`' => {
                    is_intra_inline_code = !is_intra_inline_code;
                }
                _ => {

                }
            }
            None
        }));

        let last_idx = line.chars().count() as isize;
        if Some(&last_idx) != v.last() {
            v.push(last_idx);
        }

        v.into_iter().tuple_windows().enumerate().map(|(i,(start,end))| {
            let start = std::cmp::min(start+1,end) as usize;
            let end = end as usize;
            if i % 2 == 0 {
                let elm = &line[start..end as usize];
                return Ok(elm.to_string());
            }
            let elm = &line[start..end];

            let generated_out = if elm.starts_with("ref:") {
                let elms = elm.split(":").skip(1).collect::<Vec<&str>>();

                // we expect a type and reference name
                if elms.len() != 2 {
                    return Ok(elm.to_string());
                }

                match &elms[..] {
                    ["fig", refere] => {
                        references.get::<str>(refere)
                            .ok_or(Error::InvalidReference(format!("could not find reference to `{}` in line {}", elms[1], line_num)))
                            .map(|x| format!("<a class=\"fig_ref\" href='#{}'>{}</a>", elms[1], x))
                    },
                    ["bib", refere] => {
                        references.get::<str>(refere)
                            .ok_or(Error::InvalidReference(format!("could not find reference to `{}` in line {}", elms[1], line_num)))
                            .map(|x| format!("<a class=\"bib_ref\" href='bibliography.html#{}'>{}</a>", elms[1], x))
                    },
                    ["equ", refere] => {
                        references.get::<str>(refere)
                            .ok_or(Error::InvalidReference(format!("could not find reference to `{}` in line {}", elms[1], line_num)))
                            .map(|x| format!("<a class=\"equ_ref\" href='#{}'>Eq. ({})</a>", elms[1], x))
                    },
                    [kind, _] => Err(Error::InvalidReference(format!("unknown reference type of `{}` in line {}", kind, line_num))),
                    _ =>         Err(Error::InvalidReference(format!("reference has wrong number of arguments `{}` in line {}", elms.len(), line_num)))

                }
            } else {
                fragments::parse_equation(fragment_path, elm, 1.3)
                    .map(|filename| {
                        let res = format!("<object class=\"equation_inline\" data=\"assets/{}\" type=\"image/svg+xml\"></object>", filename);
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
