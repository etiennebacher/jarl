use crate::location::Location;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::traits::ArgumentListExt;
use crate::utils::find_row_col;
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;
use anyhow::{Context, Result};
use biome_rowan::AstNode;

pub struct Lengths;

impl Violation for Lengths {
    fn name(&self) -> String {
        "lengths".to_string()
    }
    fn body(&self) -> String {
        "Use `lengths()` to find the length of each element in a list.".to_string()
    }
}

impl LintChecker for Lengths {
    fn check(
        &self,
        ast: &RSyntaxNode,
        loc_new_lines: &[usize],
        file: &str,
    ) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        let call = RCall::cast(ast.clone());
        if call.is_none() {
            return Ok(diagnostics);
        }
        let RCallFields { function, arguments } = call.unwrap().as_fields();
        let function = function?;

        let funs_to_watch = ["sapply", "vapply", "map_dbl", "map_int"];
        if !funs_to_watch.contains(&function.text().as_str()) {
            return Ok(diagnostics);
        }

        let arguments = arguments?.items();
        let arg_x = arguments.get_arg_by_name_then_position("x", 0);
        let arg_fun = arguments.get_arg_by_name_then_position("FUN", 1);

        if let Some(arg_fun) = arg_fun {
            if arg_fun
                .value()
                .context("Found named argument without any value")?
                .text()
                == "length"
            {
                let (row, column) = find_row_col(ast, loc_new_lines);
                let range = ast.text_trimmed_range();
                diagnostics.push(Diagnostic {
                    message: Lengths.into(),
                    filename: file.into(),
                    location: Location { row, column },
                    fix: Fix {
                        content: format!("lengths({})", arg_x.unwrap().text()),
                        start: range.start().into(),
                        end: range.end().into(),
                    },
                })
            }
        };

        Ok(diagnostics)
    }
}
