use crate::location::Location;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::traits::ArgumentListExt;
use crate::utils::find_row_col;
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;
use anyhow::{Context, Result};
use biome_rowan::AstNode;

pub struct ExpectLength;

impl Violation for ExpectLength {
    fn name(&self) -> String {
        "expect_length".to_string()
    }
    fn body(&self) -> String {
        "`expect_length(x, n)` is better than `expect_equal(length(x), n)`.".to_string()
    }
}

impl LintChecker for ExpectLength {
    fn check(
        &self,
        ast: &RSyntaxNode,
        loc_new_lines: &[usize],
        file: &str,
    ) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = vec![];

        // Check that the call is expect_equal / expect_identical ------------

        let call = RCall::cast(ast.clone());
        if call.is_none() {
            return Ok(diagnostics);
        }
        let RCallFields { function, arguments } = call.unwrap().as_fields();
        let function = function?;

        let funs_to_watch = ["expect_equal", "expect_identical"];
        if !funs_to_watch.contains(&function.text().as_str()) {
            return Ok(diagnostics);
        }
        let arguments = arguments?.items();

        // Check that arg `object` is length() ------------

        let arg_obj = arguments
            .get_arg_by_name_then_position("object", 0)
            .context("Missing argument `object`")?;

        let call_obj = RCall::cast(arg_obj.syntax().first_child().unwrap().clone());
        if call_obj.is_none() {
            return Ok(diagnostics);
        }
        let fields_obj = call_obj.unwrap().as_fields();
        let function_obj = fields_obj.function?;
        let arg_obj = fields_obj.arguments?.items();
        if function_obj.text() != "length" {
            return Ok(diagnostics);
        }

        // Check that arg `expected` is literal number ------------

        let arg_exp = arguments
            .get_arg_by_name_then_position("expected", 1)
            .context("Missing argument `expected`")?
            .as_fields()
            .value
            .context("Couldn't get value of argument `expected`")?;

        if arg_exp.syntax().kind() != RSyntaxKind::R_DOUBLE_VALUE
            && arg_exp.syntax().kind() != RSyntaxKind::R_INTEGER_VALUE
        {
            return Ok(diagnostics);
        }

        let (row, column) = find_row_col(ast, loc_new_lines);
        let range = ast.text_trimmed_range();
        diagnostics.push(Diagnostic {
            message: ExpectLength.into(),
            filename: file.into(),
            location: Location { row, column },
            fix: Fix {
                content: format!("expect_length({}, {})", arg_obj.text(), arg_exp.text()),
                start: range.start().into(),
                end: range.end().into(),
            },
        });
        Ok(diagnostics)
    }
}
