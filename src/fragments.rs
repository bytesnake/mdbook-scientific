use fs_err as fs;
use itertools::Itertools;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{io::Write, str, usize};

use sha2::{Digest, Sha256};

use crate::errors::*;
use crate::types::*;

/// Convert input string to 24 character hash
pub fn hash(input: impl AsRef<str>) -> String {
    let mut sh = Sha256::new();
    sh.update(input.as_ref().as_bytes());
    let mut out = format!("{:x}", sh.finalize());
    out.truncate(24);
    out
}

fn find_binary(name: &str) -> Result<std::path::PathBuf> {
    which::which(name).map_err(|error| Error::BinaryNotFound {
        binary: name.to_owned(),
        error,
    })
}

/// Generate SVG file from latex file with given zoom
pub fn generate_svg_from_latex(path: &Path, zoom: f32) -> Result<()> {
    let dest_path = path.parent().expect("Parent path must exist. qed");
    let file: &Path = path.file_name().unwrap().as_ref();

    // use latex to generate a dvi
    let dvi_path = path.with_extension("dvi");
    if !dvi_path.exists() {
        let latex_path = find_binary("latex")?;

        let cmd = Command::new(latex_path)
            .current_dir(dest_path)
            //.arg("--jobname").arg(&dvi_path)
            .arg(&file.with_extension("tex"))
            .output()
            .expect("Could not spawn latex");

        if !cmd.status.success() {
            let buf = String::from_utf8_lossy(&cmd.stdout);

            // latex prints error to the stdout, if this is empty, then something is fundamentally
            // wrong with the latex binary (for example shared library error). In this case just
            // exit the program
            if buf.is_empty() {
                let buf = String::from_utf8_lossy(&cmd.stderr);
                panic!("latex exited with `{}`", buf);
            }

            let err = buf
                .split('\n')
                .filter(|x| {
                    (x.starts_with("! ") || x.starts_with("l.")) && !x.contains("Emergency stop")
                })
                .fold(("", "", usize::MAX), |mut err, elm| {
                    if let Some(striped) = elm.strip_prefix("! ") {
                        err.0 = striped;
                    } else if let Some(striped) = elm.strip_prefix("l.") {
                        let mut elms = striped.splitn(2, ' ').map(|x| x.trim());
                        if let Some(Ok(val)) = elms.next().map(|x| x.parse::<usize>()) {
                            err.2 = val;
                        }
                        if let Some(val) = elms.next() {
                            err.1 = val;
                        }
                    }

                    err
                });

            return Err(Error::InvalidMath(
                err.0.to_string(),
                err.1.to_string(),
                err.2,
            ));
        }
    }

    // convert the dvi to a svg file with the woff font format
    let svg_path = path.with_extension("svg");
    if !svg_path.exists() && dvi_path.exists() {
        let dvisvgm_path = find_binary("dvisvgm")?;

        let cmd = Command::new(dvisvgm_path)
            .current_dir(dest_path)
            .arg("-b")
            .arg("1")
            .arg("--font-format=woff")
            .arg(&format!("--zoom={}", zoom))
            .arg(&dvi_path)
            .output()
            .expect("Couldn't run svisvgm properly!");

        let buf = String::from_utf8_lossy(&cmd.stderr);
        if !cmd.status.success() || buf.contains("error:") {
            return Err(Error::InvalidDvisvgm(buf.to_string()));
        }
    }

    Ok(())
}

/// Generate latex file from gnuplot
///
/// This function generates a latex file with gnuplot `epslatex` backend and then source it into
/// the generate latex function
fn generate_latex_from_gnuplot<'a>(
    dest_path: &Path,
    content: &Content<'a>,
    filename: &str,
) -> Result<()> {
    let content = content.as_ref();
    let gnuplot_path = find_binary("gnuplot")?;

    let cmd = Command::new(gnuplot_path)
        .stdin(Stdio::piped())
        .current_dir(dest_path)
        .arg("-p")
        .spawn()?;

    let mut stdin = cmd.stdin.expect("Stdin of gnuplot spawn must exist. qed");

    stdin.write_all(format!("set output '{}.tex'\n", filename).as_bytes())?;
    stdin.write_all("set terminal epslatex color standalone\n".as_bytes())?;
    stdin.write_all(content.as_bytes())?;

    Ok(())
}

/// Parse an equation with the given zoom
pub fn generate_replacement_file_from_template<'a>(
    dest_path: &Path,
    content: &Content<'a>,
    zoom: f32,
) -> Result<Replacement<'a>> {
    let name = hash(content);
    let path = dest_path.join(&name);

    eprintln!(
        r#"Found equation from {}:{}..{}:{}:
    {}"#,
        content.start.lineno,
        content.start.column,
        content.end.lineno,
        content.end.column,
        content.s
    );

    let tex = content.as_ref();
    // create a new tex file containing the equation
    if !path.with_extension("tex").exists() {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path.with_extension("tex"))?;

        let fragment = include_str!("fragment.tex")
            .split("$$")
            .enumerate()
            .map(|(idx, s)| match idx {
                0 | 2 => s,
                1 => tex,
                _ => unreachable!("fragment.tex must have exactly 2 instances of `$$`"),
            })
            .join("$$");

        file.write_all(fragment.as_bytes())?;
    }

    generate_svg_from_latex(&path, zoom)?;

    Ok(Replacement {
        content: content.clone(),
        intermediate: None,
        svg: PathBuf::from(name + ".svg"),
    })
}

/// Parse a latex content and convert it to a SVG file
pub fn parse_latex<'a>(dest_path: &Path, content: &Content<'a>) -> Result<Replacement<'a>> {
    let tex = content.as_ref();
    let name = hash(tex);
    let path = dest_path.join(&name);

    // create a new tex file containing the equation
    if !path.with_extension("tex").exists() {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path.with_extension("tex"))?;

        file.write_all(tex.as_bytes())?;
    }

    generate_svg_from_latex(&path, 1.0)?;

    Ok(Replacement {
        content: content.clone(),
        intermediate: None,
        svg: PathBuf::from(name + ".svg"),
    })
}

/// Parse a gnuplot file and generate a SVG file
pub fn parse_gnuplot<'a>(dest_path: &Path, content: &Content<'a>) -> Result<Replacement<'a>> {
    let name = hash(content);
    let path = dest_path.join(&name);

    if !path.with_extension("tex").exists() {
        //let name_plot = format!("{}_plot", name);
        generate_latex_from_gnuplot(dest_path, content, &name)?;
    }

    if !path.with_extension("svg").exists() {
        generate_svg_from_latex(&path, 1.0)?;
    }

    let intermediate = fs::read_to_string(path.with_extension("tex"))?;

    Ok(Replacement {
        content: content.to_owned(),
        intermediate: Some(intermediate),
        svg: PathBuf::from(name + ".svg"),
    })
}

/// Parse gnuplot without using the latex backend
pub fn parse_gnuplot_only<'a>(dest_path: &Path, content: &Content<'a>) -> Result<Replacement<'a>> {
    let gnuplot_input = content.as_ref();
    let name = hash(gnuplot_input);
    let path = dest_path.join(&name);

    if !path.with_extension("svg").exists() {
        let gnuplot_path = find_binary("gnuplot")?;
        let cmd = Command::new(gnuplot_path)
            .stdin(Stdio::piped())
            .current_dir(dest_path)
            .arg("-p")
            .spawn()
            .unwrap();
        //.expect("Could not spawn gnuplot");

        let mut stdin = cmd.stdin.unwrap();
        stdin.write_all(format!("set output '{}.svg'\n", name).as_bytes())?;
        stdin.write_all("set terminal svg\n".as_bytes())?;
        stdin.write_all("set encoding utf8\n".as_bytes())?;
        stdin.write_all(gnuplot_input.as_bytes())?;
    }

    Ok(Replacement {
        content: content.clone(),
        intermediate: None,
        svg: PathBuf::from(name + ".svg"),
    })
}

/// Generate html from BibTeX file using `bib2xhtml`
pub fn bib_to_html(source: &str, bib2xhtml: &str) -> Result<String> {
    let source = fs::canonicalize(source).unwrap();
    let bib2xhtml = Path::new(bib2xhtml);

    //./bib2xhtml.pl -s alpha -u -U ~/Documents/Bachelor_thesis/literature.bib
    let cmd = Command::new(bib2xhtml.join("./bib2xhtml.pl"))
        .current_dir(bib2xhtml)
        .args(["-s", "alpha", "-u", "-U"])
        .arg(source)
        .output()
        .expect("Could not spawn bib2xhtml");

    let buf = String::from_utf8_lossy(&cmd.stdout);

    let err_str = String::from_utf8_lossy(&cmd.stderr);
    if err_str.contains("error messages)") {
        Err(Error::InvalidBibliography(err_str.to_string()))
    } else {
        let buf = buf
            .split('\n')
            .skip_while(|x| *x != "<dl class=\"bib2xhtml\">")
            .take_while(|x| *x != "</dl>")
            .map(|x| x.replace("<a name=\"", "<a id=\""))
            .collect();

        Ok(buf)
    }
}
