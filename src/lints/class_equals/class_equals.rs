use crate::location::Location;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::utils::{find_row_col, get_first_arg, node_is_in_square_brackets};
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;
use anyhow::Result;
use biome_rowan::AstNode;

pub struct ClassEquals;

impl Violation for ClassEquals {
    fn name(&self) -> String {
        "class_equals".to_string()
    }
    fn body(&self) -> String {
        "Use `inherits(x, 'class')` instead of comparing `class(x)` with `==` or `%in%`."
            .to_string()
    }
}

impl LintChecker for ClassEquals {
    fn check(
        &self,
        ast: &RSyntaxNode,
        loc_new_lines: &[usize],
        file: &str,
    ) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        let bin_expr = RBinaryExpression::cast(ast.clone());

        if bin_expr.is_none() || node_is_in_square_brackets(ast) {
            return Ok(diagnostics);
        }

        let RBinaryExpressionFields { left, operator, right } = bin_expr.unwrap().as_fields();

        let operator = operator.unwrap();

        if operator.kind() != RSyntaxKind::EQUAL2
            && operator.kind() != RSyntaxKind::NOT_EQUAL
            && operator.text_trimmed() != "%in%"
        {
            return Ok(diagnostics);
        };

        let lhs = left?.into_syntax();
        let rhs = right?.into_syntax();

        let left_is_class = lhs
            .first_child()
            .map(|x| x.text_trimmed() == "class")
            .unwrap_or(false);
        let right_is_class = rhs
            .first_child()
            .map(|x| x.text_trimmed() == "class")
            .unwrap_or(false);
        let left_is_string = lhs.kind() == RSyntaxKind::R_STRING_VALUE;
        let right_is_string = rhs.kind() == RSyntaxKind::R_STRING_VALUE;

        if (!left_is_class && !right_is_class) || (!left_is_string && !right_is_string) {
            return Ok(diagnostics);
        }

        let fun_name =
            if operator.kind() == RSyntaxKind::EQUAL2 || operator.text_trimmed() == "%in%" {
                "inherits"
            } else {
                "!inherits"
            };

        let fun_content;
        let class_name;

        if left_is_class {
            fun_content = get_first_arg(&lhs).map(|x| x.text_trimmed());
            class_name = rhs.text_trimmed();
        } else {
            fun_content = get_first_arg(&rhs).map(|x| x.text_trimmed());
            class_name = lhs.text_trimmed();
        };

        let (row, column) = find_row_col(ast, loc_new_lines);
        let range = ast.text_trimmed_range();
        diagnostics.push(Diagnostic {
            message: ClassEquals.into(),
            filename: file.into(),
            location: Location { row, column },
            fix: Fix {
                content: format!("{}({}, {})", fun_name, fun_content.unwrap(), class_name),
                start: range.start().into(),
                end: range.end().into(),
            },
        });
        Ok(diagnostics)
    }
}
