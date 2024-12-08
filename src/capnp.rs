use anyhow::{bail, Context, Result};
use lsp_types::{Diagnostic, DiagnosticSeverity, Range, Url};

pub fn diags(uri: &Url, proto_paths: &Vec<std::path::PathBuf>) -> Result<Vec<Diagnostic>> {
    if uri.scheme() != "file" {
        bail!("Unsupported URI scheme {uri}");
    }

    let Ok(path) = uri.to_file_path() else {
        bail!("Failed to normalize URI path: {uri}");
    };

    let mut cmd = std::process::Command::new("capnp");
    let path = path
        .to_str()
        .with_context(|| format!("Non-unicode path: {path:?}"))?;
    cmd.arg("compile")
        // Add include paths.
        .args(
            proto_paths
                .iter()
                .filter_map(|p| {
                    p.to_str().or_else(|| {
                        log::warn!("Non-unicode path: {p:?}");
                        None
                    })
                })
                .map(|p| "-I".to_string() + p),
        )
        // Add the file we're compiling
        .arg(path);

    log::debug!("Running capnp: {cmd:?}");
    let output = cmd.output()?;

    log::debug!("Capnp exited: {output:?}");
    let stderr = std::str::from_utf8(output.stderr.as_slice())?;

    let res = stderr.lines().filter_map(|l| parse_diag(l)).collect();
    log::trace!("Generated diagnostics: {res:?}");
    Ok(res)
}

// Parse a single error line from the capnp parser into a diagnostic.
// Lines look like:
// foo.capnp:3:9: error: Parse error.
fn parse_diag(diag: &str) -> Option<lsp_types::Diagnostic> {
    let (_, rest) = diag.split_once(':')?;
    let (lineno, rest) = rest.split_once(':')?;
    let (colno, rest) = rest.split_once(':')?;
    let msg = rest.strip_prefix(" error: ")?.trim().trim_end_matches(".");

    // Lines from capnp stderr are 1-indexed.
    let lineno = lineno.parse::<u32>().unwrap().saturating_sub(1);
    let (col_start, col_end) = match colno.split_once('-') {
        Some((start, end)) => (start.parse::<u32>().unwrap(), end.parse::<u32>().unwrap()),
        None => {
            let start = colno.parse::<u32>().unwrap();
            (start, start)
        }
    };
    // Columns are 1-indexed as well
    let col_start = col_start.saturating_sub(1);
    let col_end = col_end.saturating_sub(1);

    Some(lsp_types::Diagnostic {
        range: Range {
            start: lsp_types::Position {
                line: lineno,
                character: col_start.try_into().ok()?,
            },
            end: lsp_types::Position {
                line: lineno,
                character: col_end.try_into().ok()?,
            },
        },
        severity: Some(if msg.ends_with("originally used here") {
            DiagnosticSeverity::HINT
        } else {
            DiagnosticSeverity::ERROR
        }),
        source: Some(String::from("capnls")),
        message: msg.into(),
        ..Default::default()
    })
}

#[test]
fn test_parse_diag() {
    assert_eq!(
        parse_diag("foo.capnp:32:9: error: Parse error.",),
        Some(lsp_types::Diagnostic {
            range: Range {
                start: lsp_types::Position {
                    line: 31,
                    character: 8,
                },
                end: lsp_types::Position {
                    line: 31,
                    character: 8,
                },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some(String::from("capnls")),
            message: "Parse error".into(),
            ..Default::default()
        })
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::Position;
    use pretty_assertions::assert_eq;

    fn capnp_file(tmp: &tempfile::TempDir, path: &str, lines: &[&str]) -> (Url, String) {
        let path = tmp.path().join(path);
        let text = lines.join("\n") + "\n";
        std::fs::write(&path, &text).unwrap();
        (Url::from_file_path(path).unwrap(), text)
    }

    #[test]
    fn test_errors() {
        let _ = env_logger::builder().is_test(true).try_init();
        let tmp = tempfile::tempdir().unwrap();

        let (uri, _) = capnp_file(
            &tmp,
            "foo.capnp",
            &[
                "@0xeb77878e33236528;",
                "struct Foo {",
                "i @0 :Int32;",
                "u @0 :Int32;",
                "one_two @1 :Int32;",
                "unknown @2 :Unknown;",
                "}",
            ],
        );

        let diags = diags(&uri, &vec![tmp.path().to_path_buf()]).unwrap();

        let expected = [
            Diagnostic {
                range: Range {
                    start: Position {
                        line: 4,
                        character: 0,
                    },
                    end: Position {
                        line: 4,
                        character: 7,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("capnls".into()),
                message: "Cap'n Proto declaration names should use camelCase and must not contain underscores. (Code generators may convert names to the appropriate style for the target language.)".into(),
                ..Default::default()
            },
            Diagnostic {
                range: Range {
                    start: Position {
                        line: 3,
                        character: 3,
                    },
                    end: Position {
                        line: 3,
                        character: 4,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("capnls".into()),
                message: "Duplicate ordinal number".into(),
                ..Default::default()
            },
            Diagnostic {
                range: Range {
                    start: Position {
                        line: 2,
                        character: 3,
                    },
                    end: Position {
                        line: 2,
                        character: 4,
                    },
                },
                severity: Some(DiagnosticSeverity::HINT),
                source: Some("capnls".into()),
                message: "Ordinal @0 originally used here".into(),
                ..Default::default()
            },
            Diagnostic {
                range: Range {
                    start: lsp_types::Position {
                        line: 5,
                        character: 12,
                    },
                    end: lsp_types::Position {
                        line: 5,
                        character: 19,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("capnls".into()),
                message: "Not defined: Unknown".into(),
                ..Default::default()
            },
        ];
        assert_eq!(diags, expected);
    }
}
