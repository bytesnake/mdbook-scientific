use nom_bibtex::error::BibtexError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    SemVer(#[from] semver::Error),

    #[error("Rendered `{0}` not supported")]
    RendererNotSupported(String),

    #[error("Invalid math: {0} {1} at line {2}")]
    InvalidMath(String, String, usize),

    #[error("Invalid reference: {0}")]
    InvalidReference(String),

    #[error("Invalid bibliography: {0}")]
    InvalidBibliography(String),

    #[error("Invalid dvi svgm: {0}")]
    InvalidDvisvgm(String),

    #[error("Binary \"{binary}\" was not found using `which`")]
    BinaryNotFound {
        binary: String,
        #[source]
        error: which::Error,
    },

    #[error("Uneven number of dollar signs found")]
    UnevenNumberDollar,

    #[error("Key section not found")]
    KeySectionNotFound,

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Bibliography {0}")]
    BibliographyMissing(String),

    #[error(transparent)]
    BibliographyParsingFailed(#[from] BibtexError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    MdBook(#[from] mdbook::errors::Error),
}
