use std::collections::HashMap;
use std::path::Path;

use crate::fragments;
use crate::error::{Error, Result};

pub fn replace_blocks(fragment_path: &Path, source: &str, head_num: &str, used_fragments: &mut Vec<String>, references: &mut HashMap<String, String>) -> Result<String> {
    let mut content = String::new();
    let mut start_line: Option<String> = None;
    let mut figures_counter = 0;
    let mut equations_counter = 0;

    source.split("\n")
    .filter_map(|line| {
        if !line.starts_with("$$") {
            if start_line.is_some() {
                content.push_str(line);
                content.push('\n');
                return None;
            } else {
                return Some(Ok(line.into()));
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
                            references.insert(refer.clone(), format!("Figure {}{}", head_num, figures_counter));

                            format!("<figure id=\"{}\" class=\"figure\"><object data=\"/assets/{}\" type=\"image/svg+xml\"/></object><figcaption>Figure {}{} {}</figcaption></figure>", refer, file, head_num, figures_counter, title)
                        })
                },
                Some("$$gnuplot") => {
                    figures_counter += 1;
                    fragments::parse_gnuplot(fragment_path, elms.map(|x| x.to_string()).collect(), &content)
                        .map(|(file, title, refer)| {
                            used_fragments.push(file.clone());

                            references.insert(refer.clone(), format!("Figure {}{}", head_num, figures_counter));

                            format!("<figure id=\"{}\" class=\"figure\"><object data=\"/assets/{}\" type=\"image/svg+xml\"/></object><figcaption>Figure {}.{} {}</figcaption></figure>", refer, file, head_num, figures_counter, title)
                        })
                },
               Some("$$gnuplotonly") => {
                    figures_counter += 1;
                    fragments::parse_gnuplot_only(fragment_path,elms.map(|x| x.to_string()).collect(), &content)
                        .map(|(file,title,refer)| {
                            used_fragments.push(file.clone());

                            references.insert(refer.clone(), format!("Figure {}{}", head_num, figures_counter));

                            format!("<figure id=\"{}\" class=\"figure\"><object data=\"/assets/{}\" type=\"image/svg+xml\"/></object><figcaption>Figure {}.{} {}</figcaption></figure>", refer, file, head_num, figures_counter, title)
                        })
               },

                Some("$$equation") | _ => {
                    fragments::parse_equation(fragment_path, elms.map(|x| x.to_string()).collect(), &content, 1.6)
                        .map(|(filename, refer)| {
                            used_fragments.push(filename.clone());

                            match refer {
                                Some(refer) => {
                                    equations_counter += 1;
                                    references.insert(refer.clone(), format!("{}{}", head_num, equations_counter));

                                    format!("<div id=\"{}\" class=\"equation\"><div class=\"equation_inner\"><object data=\"/assets/{}\" type=\"image/svg+xml\"></object></div><span>({}.{})</span></div>\n", refer, filename, head_num, equations_counter)
                                },
                                None => format!("<div class=\"equation\"><div class=\"equation_inner\"><object data=\"/assets/{}\" type=\"image/svg+xml\"></object></div></div>\n", filename)
                            }
                        })
                }
            };
            content = "".into();
            start_line = None;

            Some(generated_out)
        } else {
            start_line = Some(line.to_string());
            None
        }
    })
    .collect::<Result<Vec<_>>>()
    .map(|x| x.join("\n"))
}

pub fn replace_inline_blocks(fragment_path: &Path, source: &str, references: &HashMap<String, String>, used_fragments: &mut Vec<String>) -> Result<String> {
    source.split("\n").enumerate().map(|(line_num, line)| {
        if line.matches("$").count() % 2 != 0 {
            return Err(Error::UnevenNumberDollar);
        }

        line.split("$").enumerate().map(|(i, elm)| {
            if i % 2 == 0 {
                return Ok(elm.to_string());
            }

            let generated_out = if elm.starts_with("ref") {
                let elms = elm.split(":").skip(1).collect::<Vec<&str>>();

                match &elms[..] {
                    ["fig", refere] => {
                        references.get::<str>(refere)
                            .ok_or(Error::InvalidReference(format!("could not find reference to `{}` in line {}", elms[1], line_num)))
                            .map(|x| format!("<a class=\"fig_ref\" href='#{}'>{}</a>", elms[1], x))
                    },
                    ["bib", refere] => {
                        references.get::<str>(refere)
                            .ok_or(Error::InvalidReference(format!("could not find reference to `{}` in line {}", elms[1], line_num)))
                            .map(|_| format!("<a class=\"bib_ref\" href='#{}'>{}</a>", elms[1], elms[1]))
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
              fragments::parse_equation(fragment_path, Vec::new(), elm, 1.3)
                .map(|(filename, _)| {
                    let res = format!("<object class=\"equ_inline\" data=\"/assets/{}\" type=\"image/svg+xml\"></object>", filename);
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
