use fs_err as fs;
use itertools::Itertools;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::errors::{Error, Result};
use crate::fragments;
use crate::types::*;

const BLOCK_DELIM: &str = "$$";
const INLINE_BLOCK_DELIM: &str = "$";

pub fn format_figure<'a>(
    replacement: &Replacement<'a>,
    refer: &str,
    head_num: &str,
    figures_counter: usize,
    title: &str,
    renderer: SupportedRenderer,
) -> String {
    use SupportedRenderer::*;
    match renderer {
        Html | Markdown => {
            format!(
                r#"<figure id="{refer}" class="figure">
                    <object data="assets/{file}" type="image/svg+xml"/></object>
                    <figcaption>Figure {head_num}{figures_counter} {title}</figcaption>
                </figure>"#,
                refer = refer,
                head_num = head_num,
                figures_counter = figures_counter,
                title = title,
                file = replacement.svg.display()
            )
        }
        Latex | Tectonic => {
            format!(r#"\[{}\]"#, replacement.intermediate())
        }
    }
}

pub fn format_equation_block<'a>(
    replacement: &Replacement<'a>,
    refer: &str,
    head_num: &str,
    equations_counter: usize,
    renderer: SupportedRenderer,
) -> String {
    use SupportedRenderer::*;
    match renderer {
        Html | Markdown => {
            format!(
                r#"<div id="{refer}" class="equation">
                    <div class="equation_inner">
                        <object data="assets/{file}" type="image/svg+xml"></object>
                    </div><span>({head_num}{equations_counter})</span>
                </div>"#,
                refer = refer,
                head_num = head_num,
                equations_counter = equations_counter,
                file = replacement.svg.display()
            )
        }
        Latex | Tectonic => {
            format!(r#"\[{}\]"#, replacement.intermediate())
        }
    }
}

pub fn format_equation<'a>(replacement: &Replacement<'a>, renderer: SupportedRenderer) -> String {
    use SupportedRenderer::*;
    match renderer {
        Html | Markdown => {
            format!(
                r#"<div class="equation"><div class="equation_inner"><object data="assets/{file}" type="image/svg+xml"></object></div></div>\n"#,
                file = replacement.svg.display()
            )
        }
        Latex | Tectonic => {
            format!(r#"\[{}\]"#, replacement.intermediate())
        }
    }
}

pub fn format_inline_equation<'a>(
    replacement: &Replacement<'a>,
    renderer: SupportedRenderer,
) -> String {
    use SupportedRenderer::*;
    match renderer {
        Html | Markdown => {
            format!(
                r#"<object class="equation_inline" data="assets/{file}" type="image/svg+xml"></object>"#,
                file = replacement.svg.display()
            )
        }
        Latex | Tectonic => {
            format!(r#"${}$"#, replacement.content.s)
        }
    }
}

/// Takes a file residing at path, and uses it to produce
/// includable rendered equations.
pub fn replace_blocks(
    fragment_path: &Path,
    asset_path: &Path,
    source: &str,
    head_num: &str,
    renderer: SupportedRenderer,
    used_fragments: &mut Vec<PathBuf>,
    references: &mut HashMap<String, String>,
) -> Result<String> {
    let mut content = String::new();

    let mut start_loco: Option<(LiCo, String)> = None;

    let mut figures_counter = 0;
    let mut equations_counter = 0;

    let mut add_object =
        move |replacement: &Replacement<'_>, refer: &str, title: Option<&str>| -> String {
            let file = replacement.svg.as_path();
            used_fragments.push(file.to_owned());

            if let Some(title) = title {
                figures_counter += 1;
                references.insert(
                    refer.to_string(),
                    format!("Figure {}{}", head_num, figures_counter),
                );

                format_figure(
                    replacement,
                    refer,
                    head_num,
                    figures_counter,
                    title,
                    renderer,
                )
            } else if !refer.is_empty() {
                equations_counter += 1;
                references.insert(
                    refer.to_string(),
                    format!("{}{}", head_num, equations_counter),
                );
                format_equation_block(replacement, refer, head_num, equations_counter, renderer)
            } else {
                format_equation(replacement, renderer)
            }
        };

    fs::create_dir_all(fragment_path)?;

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
                .splitn(3, ',')
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
                        .map(|ref file| add_object(file, refer, Some(title))),
                    ["gnuplot", refer, title] => fragments::parse_gnuplot(fragment_path, &content)
                        .map(|ref file| add_object(file, refer, Some(title))),
                    ["gnuplotonly", refer, title] => {
                        fragments::parse_gnuplot_only(fragment_path, &content)
                            .map(|ref file| add_object(file, refer, Some(title)))
                    }

                    ["equation", refer] | ["equ", refer] => {
                        fragments::generate_replacement_file_from_template(
                            fragment_path,
                            &content,
                            1.6,
                        )
                        .map(|ref file| add_object(file, refer, None))
                    }

                    ["equation"] | ["equ"] | _ => {
                        fragments::generate_replacement_file_from_template(
                            fragment_path,
                            &content,
                            1.6,
                        )
                        .map(|ref file| add_object(file, "", None))
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

/// Currently there is no way to display mermaid
/// TODO FIXME
pub fn gen_mermaid_charts(source: &str, renderer: SupportedRenderer) -> Result<String> {
    match renderer {
        // markdown and html can just fine deal with it
        SupportedRenderer::Html => return Ok(source.to_owned()),
        SupportedRenderer::Markdown => return Ok(source.to_owned()),
        _ => {
            eprintln!("Stripping `mermaid` fencing of code block, not supported yet")
        }
    }

    use pulldown_cmark::*;
    use pulldown_cmark_to_cmark::cmark;

    let mut buf = String::with_capacity(source.len());

    let events = Parser::new_ext(&source, Options::all())
        .into_offset_iter()
        .filter_map(|(mut event, _offset)| {
            match event {
                Event::Start(Tag::CodeBlock(ref mut kind)) => match kind {
                    CodeBlockKind::Fenced(s) if s.as_ref() == "mermaid" => {
                        *kind = CodeBlockKind::Fenced("text".into());
                    }
                    _ => {}
                },
                Event::End(Tag::CodeBlock(ref mut kind)) => match kind {
                    CodeBlockKind::Fenced(s) if s.as_ref() == "mermaid" => {
                        *kind = CodeBlockKind::Fenced("text".into());
                    }
                    _ => {}
                },
                _ => {}
            }
            Some(event)
        });

    pulldown_cmark_to_cmark::cmark(events, &mut buf).map_err(Error::CommonMarkGlue)?;
    Ok(buf)
}

pub fn replace_inline_blocks(
    fragment_path: &Path,
    source: &str,
    references: &HashMap<String, String>,
    renderer: SupportedRenderer,
    used_fragments: &mut Vec<PathBuf>,
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
            }));

            let last_idx = linecontent.chars().count() as isize;
            if Some(&last_idx) != v.last() {
                v.push(last_idx);
            }

            v.into_iter()
                .tuple_windows()
                .enumerate()
                .map(|(i, (start, end))| {
                    let start = std::cmp::min(start + 1, end) as usize;
                    let end = end as usize;

                    let mut iter = linecontent.char_indices().skip(start);

                    // zero _byte_ indices
                    // TODO FIXME
                    let (byte_start, byte_end) = iter
                        .next()
                        .map(|(byte_start, _)| {
                            (
                                byte_start,
                                iter.skip(end.saturating_sub(start).saturating_sub(1))
                                    .next()
                                    .map(|x| x.0)
                                    .unwrap_or(linecontent.len()),
                            )
                        })
                        .unwrap_or_else(|| {
                            if let Some((bytes_end, _)) = linecontent.char_indices().last() {
                                (0, bytes_end)
                            } else {
                                (0, linecontent.len())
                            }
                        });
                    let elm = dbg!(&linecontent[byte_start..byte_end]);

                    if i % 2 == 0 {
                        // not within, so just return a string
                        return Ok(elm.to_owned());
                    }

                    let content = Content {
                        // content without the $ delimiters
                        s: elm,
                        start: LiCo {
                            lineno,
                            // one indexed ?
                            column: start,
                        },
                        end: LiCo {
                            lineno,
                            column: end,
                        },
                    };

                    let generated_out =
                        if elm.starts_with("ref:") {
                            let elms = elm.split(':').skip(1).collect::<Vec<&str>>();

                            // we expect a type and reference name
                            if elms.len() != 2 {
                                // if not just return as text again
                                return Ok(elm.to_string());
                            }

                            match &elms[..] {
                            ["fig", refere] => references
                                .get::<str>(refere)
                                .ok_or(Error::InvalidReference{
                                    to: elms[1].to_owned(), lineno
                                })
                                .map(|x| {
                                    format!(r#"<a class="fig_ref" href='#{}'>{}</a>"#, elms[1], x)
                                }),
                            ["bib", refere] => references
                                .get::<str>(refere)
                                .ok_or(Error::InvalidReference{
                                    to: elms[1].to_owned(), lineno
                                })
                                .map(|x| {
                                    format!(
                                        r#"<a class="bib_ref" href='bibliography.html#{}'>{}</a>"#,
                                        elms[1], x
                                    )
                                }),
                            ["equ", refere] => references
                                .get::<str>(refere)
                                .ok_or(Error::InvalidReference{
                                    to: elms[1].to_owned(), lineno
                                })
                                .map(|x| {
                                    format!(
                                        r#"<a class="equ_ref" href='#{}'>Eq. ({})</a>"#,
                                        elms[1], x
                                    )
                                }),
                            [kind, _] => Err(Error::UnknownReferenceKind{
                                kind: kind.to_owned().to_owned(), lineno,
                            }),
                            _ => Err(Error::UnexpectedReferenceArgCount {
                                count: elms.len(),
                                lineno
                            }),
                        }
                        } else {
                            fragments::generate_replacement_file_from_template(
                                fragment_path,
                                &content,
                                1.3,
                            )
                            .map(|replacement| {
                                let res = format_inline_equation(&replacement, renderer);
                                used_fragments.push(replacement.svg);
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
