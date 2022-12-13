use clap::Parser;
use mdbook::preprocess::{CmdPreprocessor, Preprocessor};
use mdbook_scientific::errors::*;
use mdbook_scientific::Scientific;
use std::io;
use std::process;

#[derive(clap::Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = "A mdbook preprocessor which handles formulae, figures wrapped in `$` and `$$` signs"
)]
struct Args {
    #[command(subcommand)]
    supports: Option<Sub>,
}

#[derive(clap::Subcommand, Debug)]
#[doc = "Check whether a renderer is supported by this preprocessor"]
enum Sub {
    Supports { renderer: String },
}

fn main() -> color_eyre::eyre::Result<()> {
    eprintln!("INVOKE MDBOOK SCIENTIFIC!");
    color_eyre::install()?;

    let args = Args::try_parse()?;

    let preprocessor = Scientific::new();

    if let Some(Sub::Supports { ref renderer }) = args.supports {
        handle_supports(&preprocessor, renderer);
    } else {
        handle_preprocessing(&preprocessor)?;
    }
    Ok(())
}

fn handle_preprocessing(pre: &dyn Preprocessor) -> Result<()> {
    let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;

    let compiled_against = semver::VersionReq::parse(mdbook::MDBOOK_VERSION)?;
    let running_against = semver::Version::parse(ctx.mdbook_version.as_str())?;
    if !compiled_against.matches(&running_against) {
        // We should probably use the `semver` crate to check compatibility
        // here...
        eprintln!(
            "Warning: The {} plugin was built against version {} of mdbook, \
             but we're being called from version {}",
            pre.name(),
            mdbook::MDBOOK_VERSION,
            ctx.mdbook_version
        );
    }

    let processed_book = dbg!(pre.run(&ctx, dbg!(book))?);

    serde_json::to_writer(io::stdout(), &processed_book)?;

    Ok(())
}

fn handle_supports(pre: &dyn Preprocessor, renderer: impl AsRef<str>) -> ! {
    let supported = pre.supports_renderer(renderer.as_ref());

    // Signal whether the renderer is supported by exiting with 1 or 0.
    if supported {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn clap_supports() {
        assert_matches!(
            Args::try_parse_from(vec!["mdbook-scientific", "supports"]),
            Err(_)
        );
        assert_matches!(Args::try_parse_from(vec!["mdbook-scientific", "supports", "tectonic"]).unwrap(),
        Args {
            supports: Some(Sub::Supports{ renderer }),
            ..
        } => {
            assert_eq!(renderer, "tectonic");
        });
    }

    #[test]
    fn clap_supports_no_sub() {
        assert_matches!(Args::try_parse_from(vec!["mdbook-scientific"]).unwrap(),
        Args {
            supports: None,
            ..
        } => {
        });
    }
}
