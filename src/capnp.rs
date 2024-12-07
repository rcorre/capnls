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
    let lineno = lineno.parse::<u32>().unwrap() - 1;
    let (col_start, col_end) = match colno.split_once('-') {
        Some((start, end)) => (start.parse::<u32>().unwrap(), end.parse::<u32>().unwrap()),
        None => {
            let start = colno.parse::<u32>().unwrap();
            (start, start)
        }
    };

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
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some(String::from("capnls")),
        message: msg.trim().into(),
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
                    character: 9,
                },
                end: lsp_types::Position {
                    line: 31,
                    character: 9,
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
            "foo.proto",
            &[
                "syntax = \"proto3\";",
                "message Foo {",
                "int i = 1;",
                "uint32 u = 1;",
                "}",
            ],
        );

        let diags = diags(&uri, &vec![tmp.path().to_path_buf()]).unwrap();

        assert_eq!(
            diags,
            vec![
                Diagnostic {
                    range: Range {
                        start: lsp_types::Position {
                            line: 2,
                            character: 0,
                        },
                        end: lsp_types::Position {
                            line: 2,
                            character: 10,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("capnls".into()),
                    message: "\"int\" is not defined".into(),
                    ..Default::default()
                },
                Diagnostic {
                    range: Range {
                        start: lsp_types::Position {
                            line: 3,
                            character: 0,
                        },
                        end: lsp_types::Position {
                            line: 3,
                            character: 13,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("capnls".into()),
                    message: "Field number 1 has already been used in \"Foo\" by field \"i\""
                        .into(),
                    ..Default::default()
                },
            ]
        );
    }

    #[test]
    fn test_warnings() {
        let _ = env_logger::builder().is_test(true).try_init();
        let tmp = tempfile::tempdir().unwrap();

        capnp_file(&tmp, "bar.proto", &["syntax = \"proto3\";"]);

        let (uri, _) = capnp_file(
            &tmp,
            "foo.proto",
            &["syntax = \"proto3\";", "import \"bar.proto\";"],
        );

        let diags = diags(&uri, &vec![tmp.path().to_path_buf()]).unwrap();

        assert_eq!(
            diags,
            vec![Diagnostic {
                range: Range {
                    start: lsp_types::Position {
                        line: 1,
                        character: 0,
                    },
                    end: lsp_types::Position {
                        line: 1,
                        character: 19,
                    },
                },
                severity: Some(DiagnosticSeverity::WARNING),
                source: Some("capnls".into()),
                message: "Import bar.proto is unused".into(),
                ..Default::default()
            },]
        );
    }
}