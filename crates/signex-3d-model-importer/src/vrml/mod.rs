pub mod lexer;
pub mod parser;

use std::path::Path;

use crate::error::ModelImportError;
pub use parser::VrmlMesh;

/// Parse a VRML97 source file and return the flat mesh list.
///
/// # Errors
///
/// Returns [`ModelImportError`] when the source cannot be read or parsed.
pub fn load(path: &Path) -> Result<Vec<VrmlMesh>, ModelImportError> {
    let source = std::fs::read_to_string(path).map_err(|e| ModelImportError::IoFailed {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;

    let (tokens, lines) = lexer::tokenize(&source);

    parser::parse(&tokens, &lines).map_err(|e| match e {
        parser::ParseError::UnexpectedEof { line } => ModelImportError::VrmlParseFailed {
            path: path.to_path_buf(),
            line,
            reason: "unexpected end of file".to_owned(),
        },
        parser::ParseError::UnresolvedUse { name } => ModelImportError::VrmlUnresolvedUse { name },
        parser::ParseError::MalformedNumber { line, value } => ModelImportError::VrmlParseFailed {
            path: path.to_path_buf(),
            line,
            reason: format!("malformed number: {value}"),
        },
    })
}
