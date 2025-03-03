use crate::location::Location;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::utils::{find_row_col, get_args, node_is_in_square_brackets};
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct ClassEquals;

impl Violation for ClassEquals {
    fn name(&self) -> String {
        "class-equals".to_string()
    }
    fn body(&self) -> String {
        "Use `inherits(x, 'class')` instead of comparing `class(x)` with `==` or `%in%`."
            .to_string()
    }
}

impl LintChecker for ClassEquals {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &[usize], file: &str) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        let bin_expr = RBinaryExpression::cast(ast.clone());

        if bin_expr.is_none() || node_is_in_square_brackets(ast) {
            return diagnostics;
        }

        let RBinaryExpressionFields { left: _, operator, right: _ } = bin_expr.unwrap().as_fields();

        let operator = operator.unwrap();

        if operator.kind() != RSyntaxKind::EQUAL2
            && operator.kind() != RSyntaxKind::NOT_EQUAL
            && operator.text_trimmed() != "%in%"
        {
            return diagnostics;
        };

        let mut children = ast.children();
        let lhs = children.next().unwrap();
        let rhs = children.next().unwrap();

        let left_is_class = match lhs.first_child() {
            Some(x) => x.text_trimmed() == "class",
            None => false,
        };
        let right_is_class = match rhs.first_child() {
            Some(x) => x.text_trimmed() == "class",
            None => false,
        };
        let left_is_string = lhs.kind() == RSyntaxKind::R_STRING_VALUE;
        let right_is_string = rhs.kind() == RSyntaxKind::R_STRING_VALUE;

        if (!left_is_class && !right_is_class) || (!left_is_string && !right_is_string) {
            return diagnostics;
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
            fun_content = get_args(&lhs).and_then(|x| Some(x.text_trimmed()));
            class_name = rhs.text_trimmed();
        } else {
            fun_content = get_args(&rhs).and_then(|x| Some(x.text_trimmed()));
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
        diagnostics
    }
}
