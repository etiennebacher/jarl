use crate::diagnostic::*;
use crate::utils::{get_function_name, get_nested_functions_content, node_contains_comments};
use crate::utils_ast::AstNodeExt;
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for dangerous usage of `all.equal(...)`, for instance in `if()`
/// conditions or `while()` loops.
///
/// ## Why is this bad?
///
/// `all.equal()` returns `TRUE` in the absence of differences but returns a
/// character string (not `FALSE`) in the presence of differences. Usage of
/// `all.equal()` without wrapping it in `isTRUE()` are thus likely to generate
/// unexpected errors if the compared objects have differences. An alternative
/// is to use `identical()` to compare vector of strings or when exact equality
/// is expected.
///
/// This rule has automated fixes that are marked unsafe and therefore require
/// passing `--unsafe-fixes`. This is because automatically fixing those cases
/// can change the runtime behavior if some code relied on the behaviour of
/// `all.equal()` (likely by mistake).
///
/// ## Example
///
/// ```r
/// a <- 1
/// b <- 1
///
/// if (all.equal(a, b, tolerance = 1e-3)) message('equal')
/// if (all.equal(a, b)) message('equal')
/// !all.equal(a, b)
/// isFALSE(all.equal(a, b))
///
/// ```
///
/// Use instead:
/// ```r
/// a <- 1
/// b <- 1
///
/// if (isTRUE(all.equal(a, b, tolerance = 1e-3))) message('equal')
/// if (isTRUE(all.equal(a, b))) message('equal')
/// !isTRUE(all.equal(a, b))
/// !isTRUE(all.equal(a, b))
/// ```
///
/// ## References
///
/// See `?all.equal`
pub fn all_equal(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    // 1) Check for isFALSE(all.equal(...))
    if let Some((inner_content, outer_syntax)) =
        get_nested_functions_content(ast, "isFALSE", "all.equal")?
    {
        let range = outer_syntax.text_trimmed_range();
        return Ok(Some(Diagnostic::new(
            ViolationData::new(
                "all_equal".to_string(),
                "`isFALSE(all.equal())` always returns `FALSE`".to_string(),
                Some("Use `!isTRUE()` to check for differences instead.".to_string()),
            ),
            range,
            Fix {
                content: format!("!isTRUE(all.equal({inner_content}))"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(&outer_syntax),
            },
        )));
    }

    // 2) Check for other cases: if (all.equal()), while(all.equal()), etc.
    let function = ast.function()?;
    let fun_name = get_function_name(function);
    if fun_name != "all.equal" {
        return Ok(None);
    }

    if !ast.parent_is_if_condition()
        && !ast.parent_is_while_condition()
        && !ast.parent_is_bang_unary()
    {
        return Ok(None);
    }

    let msg = "If `all.equal()` is false, it will return a string and not `FALSE`.".to_string();
    let mut range = ast.syntax().text_trimmed_range();

    let fix_content = if ast.parent_is_bang_unary() {
        if let Some(prev) = ast.syntax().prev_sibling_or_token() {
            range = TextRange::new(prev.text_trimmed_range().start(), range.end())
        }
        format!("!isTRUE({})", ast.to_trimmed_text())
    } else {
        format!("isTRUE({})", ast.to_trimmed_text())
    };

    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "all_equal".to_string(),
            msg,
            Some("Wrap `all.equal()` in `isTRUE()`, or replace it by `identical()` if no tolerance is required.".to_string()),
        ),
        range,
        Fix {
            content: fix_content,
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
