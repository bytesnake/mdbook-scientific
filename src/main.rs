use clap::{App, Arg, ArgMatches, SubCommand};
use mdbook::preprocess::{CmdPreprocessor, Preprocessor};
use mdbook_scientific::error::*;
use mdbook_scientific::Scientific;
use std::io;
use std::process;

pub fn make_app() -> App<'static> {
    App::new("scientifica")
        .about("A mdbook preprocessor which handles $ signs")
        .subcommand(
            SubCommand::with_name("supports")
                .arg(Arg::with_name("renderer").required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        )
}

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    let matches = make_app().get_matches();

    let preprocessor = Scientific::new();

    if let Some(sub_args) = matches.subcommand_matches("supports") {
        handle_supports(&preprocessor, sub_args);
    } else {
        handle_preprocessing(&preprocessor).map_err(Error::from)?;
    }
    Ok(())
}

fn handle_preprocessing(pre: &dyn Preprocessor) -> Result<()> {
    eprintln!("Hey! 1");

    let (ctx, book) = CmdPreprocessor::parse_input(io::stdin()).map_err(Error::MdBook)?;

    if ctx.mdbook_version != mdbook::MDBOOK_VERSION {
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

    eprintln!("Hey! 2");

    let processed_book = pre.run(&ctx, book)?;

    eprintln!("Hey! 3");

    serde_json::to_writer(io::stdout(), &processed_book)?;

    Ok(())
}

fn handle_supports(pre: &dyn Preprocessor, sub_args: &ArgMatches) -> ! {
    let renderer = sub_args
        .value_of("renderer")
        .expect("Required argument \"renderer\" is provided by mdbook. qed");
    let supported = pre.supports_renderer(&renderer);

    // Signal whether the renderer is supported by exiting with 1 or 0.
    if supported {
        process::exit(0);
    } else {
        process::exit(1);
    }
}
