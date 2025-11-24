use crate::diagnostic::Diagnostic;
use crate::location::Location;
use air_r_syntax::{
    AnyRExpression, RArgument, RArgumentList, RCall, RCallFields, RExtractExpressionFields,
    RSyntaxNode,
};
use anyhow::{Result, anyhow};
use biome_rowan::AstNode;
use biome_rowan::AstSeparatedList;

/// Find the positions of the new line characters in the given AST.
pub fn find_new_lines(ast: &RSyntaxNode) -> Result<Vec<usize>> {
    match ast.first_child() {
        Some(rootnode) => Ok(rootnode
            .to_string()
            .match_indices("\n")
            .map(|x| x.0)
            .collect::<Vec<usize>>()),
        None => Err(anyhow!(
            "Couldn't find root node. Maybe the document contains a parsing error?"
        )),
    }
}

/// Takes the start of the range of a Diagnostic and the indices for the new
/// lines. Returns the (row, col) position of the Diagnostic in the file.
///
/// The row position is the 1 + the number of new line characters before the
/// start of the range.
/// "1 + 1\nany(is.na(x))"
/// -> there is one \n so this diagnostic appears on line 2.
///
/// The col position is the number of characters between the start of the range
/// and the last new line character before the start of the range.
/// "1 + 1\nany(is.na(x))"
/// -> the range of the diagnostic starts immediately following \n so it's in
///    column 0
///
/// Note that the row position is 1-indexed but the column position is 0-indexed.
pub fn find_row_col(start: usize, loc_new_lines: &[usize]) -> (usize, usize) {
    let new_lines_before = loc_new_lines
        .iter()
        .filter(|x| *x <= &start)
        .collect::<Vec<&usize>>();
    let n_new_lines = new_lines_before.len();
    let last_new_line = match new_lines_before.last() {
        Some(x) => **x,
        None => 0_usize,
    };

    let col: usize = if last_new_line == 0 {
        start
    } else {
        start - last_new_line - 1
    };
    let row: usize = n_new_lines + 1;
    (row, col)
}

/// Takes a vector of `Diagnostic`s, all of which come with a range, and convert
/// this range into actual (row, col) location using the position of new lines.
pub fn compute_lints_location(
    diagnostics: Vec<Diagnostic>,
    loc_new_lines: &[usize],
) -> Vec<Diagnostic> {
    diagnostics
        .into_iter()
        .map(|mut diagnostic| {
            let start: usize = diagnostic.range.start().into();
            let loc = find_row_col(start, loc_new_lines);
            diagnostic.location = Some(Location::new(loc.0, loc.1));
            diagnostic
        })
        .collect()
}

/// Takes a list of arguments and returns all the unnamed ones (mostly used when a function has `...`).
pub fn get_unnamed_args(args: &RArgumentList) -> Vec<RArgument> {
    args.into_iter()
        .filter_map(|x| {
            let arg = x.clone().unwrap();
            (arg.name_clause().is_none()).then_some(arg)
        })
        .collect()
}

/// Takes a list of arguments and returns all the named ones.
pub fn get_named_args(args: &RArgumentList) -> Vec<RArgument> {
    args.into_iter()
        .filter_map(|x| {
            let arg = x.clone().unwrap();
            (arg.name_clause().is_some()).then_some(arg)
        })
        .collect()
}

/// Takes a list of arguments and tries to extract the one named `name`.
pub fn get_arg_by_name(args: &RArgumentList, name: &str) -> Option<RArgument> {
    args.into_iter()
        .find(|x| {
            let name_clause = x.clone().unwrap().name_clause();
            if let Some(name_clause) = name_clause {
                match name_clause.name() {
                    Ok(name_clause) => name_clause.to_string().trim() == name,
                    _ => false,
                }
            } else {
                false
            }
        })
        .map(|x| x.unwrap())
}

/// Takes a list of arguments and tries to extract the one in position `pos`.
/// Argument `pos` is 1-indexed.
pub fn get_arg_by_position(args: &RArgumentList, pos: usize) -> Option<RArgument> {
    args.iter().nth(pos - 1).map(|x| x.unwrap())
}

/// Takes a list of arguments and first tries to extract the one named `name`.
/// If it doesn't find any argument with this name, it tries to get the
/// argument in position `pos`.
/// Returns None if this second attempt also fails.
/// Argument `pos` is 1-indexed.
pub fn get_arg_by_name_then_position(
    args: &RArgumentList,
    name: &str,
    pos: usize,
) -> Option<RArgument> {
    match get_arg_by_name(args, name) {
        Some(by_name) => Some(by_name),
        _ => get_arg_by_position(args, pos),
    }
}

/// Checks whether an argument named `name` or in position `pos` exist in the
/// argument list passed as input.
pub fn is_argument_present(args: &RArgumentList, name: &str, position: usize) -> bool {
    get_arg_by_name_then_position(args, name, position).is_some()
}

/// Takes a list of arguments and removes the one that is named `name` or the
/// one in position `pos` if no argument was found in the first step.
pub fn drop_arg_by_name_or_position(
    args: &RArgumentList,
    name: &str,
    pos: usize,
) -> Option<Vec<RArgument>> {
    let mut dropped_by_name = false;

    let by_name: Vec<RArgument> = args
        .iter()
        .filter_map(|arg| {
            let arg = arg.clone().unwrap();
            if let Some(name_clause) = arg.name_clause()
                && let Ok(n) = name_clause.name()
                && n.to_string().trim() == name
            {
                dropped_by_name = true;
                return None;
            }
            Some(arg)
        })
        .collect();

    if dropped_by_name {
        return Some(by_name);
    }

    let by_pos: Vec<RArgument> = args
        .iter()
        .enumerate()
        .filter_map(|(i, arg)| {
            if i == pos - 1 {
                return None;
            }
            Some(arg.clone().unwrap())
        })
        .collect();

    if by_pos.len() != args.len() {
        Some(by_pos)
    } else {
        None
    }
}

/// Return the function name of an expression. This takes AnyRExpression because
/// multiple cases are possible:
/// - fun() -> "fun"
/// - foo::fun() -> "fun"
/// - self$fun() -> "fun"
/// - return() -> "return"
pub fn get_function_name(function: AnyRExpression) -> String {
    let fn_name = if let Some(ns_expr) = function.as_r_namespace_expression() {
        if let Ok(expr) = ns_expr.right() {
            if let Some(id) = expr.as_r_identifier() {
                if let Ok(token) = id.name_token() {
                    let trimmed = token.token_text_trimmed();
                    Some(trimmed.text().to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else if let Some(extract_expr) = function.as_r_extract_expression() {
        let RExtractExpressionFields { left, right, operator } = extract_expr.as_fields();

        if let Ok(left) = left
            && let Ok(right) = right
            && let Ok(operator) = operator
        {
            if let Some(left_id) = left.as_r_identifier()
                && let Some(right_id) = right.as_r_identifier()
            {
                if let Ok(left_token) = left_id.name_token()
                    && let Ok(right_token) = right_id.name_token()
                {
                    let left_trimmed = left_token.token_text_trimmed();
                    let right_trimmed = right_token.token_text_trimmed();
                    Some(
                        [
                            left_trimmed.text().to_string(),
                            operator.text_trimmed().to_string(),
                            right_trimmed.text().to_string(),
                        ]
                        .join(""),
                    )
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else if function.as_r_return_expression().is_some() {
        Some("return".to_string())
    } else if let Some(id) = function.as_r_identifier() {
        if let Ok(token) = id.name_token() {
            let trimmed = token.token_text_trimmed();
            Some(trimmed.text().to_string())
        } else {
            None
        }
    } else {
        None
    };

    // TODO: self$foo() is handled but not in a recursive way so self$bar$foo()
    // errors for instance.
    // Those function names shouldn't trigger lint rules so fixing this is not
    // urgent.
    fn_name.unwrap_or("".to_string())
}

// Checks if an Rcall corresponds to a nested function of the form
// `outer_fn(inner_fn(content))`. If so, it returns `content`, otherwise None.
pub fn get_nested_functions_content(
    call: &RCall,
    outer_fn: &str,
    inner_fn: &str,
) -> Result<Option<String>> {
    let RCallFields { function, arguments } = call.as_fields();

    let function = function?;
    let outer_fn_name = get_function_name(function);

    if outer_fn_name != outer_fn {
        return Ok(None);
    }

    let items = arguments?.items();

    let unnamed_arg = items
        .into_iter()
        .find(|x| x.clone().unwrap().name_clause().is_none());

    // any(na.rm = TRUE/FALSE) and any() are valid
    if unnamed_arg.is_none() {
        return Ok(None);
    }

    let value = unnamed_arg.unwrap()?.value();

    if let Some(inner) = value
        && let Some(inner2) = inner.as_r_call()
    {
        let RCallFields { function, arguments } = inner2.as_fields();

        let function = function?;
        let inner_fn_name = get_function_name(function);

        if inner_fn_name != inner_fn {
            return Ok(None);
        }

        let inner_content = arguments?.items().into_syntax().to_string();

        Ok(Some(inner_content))
    } else {
        Ok(None)
    }
}

/// Checks if a syntax node contains comments somewhere between subnodes.
/// This is used to not provide a fix when comments are present to avoid
/// destroying them.
///
/// This returns `false` if the comment is leading or trailing because we want
/// to catch cases like:
/// ```r,ignore
/// any(
///   # comment 1
///   is.na(
///     # comment 2
///     x
///   )
/// )
/// ```
/// and not cases like
/// ```r,ignore
/// # comment 1
/// any(is.na(x))
///
/// any(is.na(x)) # comment 2
/// ```
pub fn node_contains_comments(node: &air_r_syntax::RSyntaxNode) -> bool {
    (node.has_comments_direct() || node.has_comments_descendants())
        && !node.has_trailing_comments()
        && !node.has_leading_comments()
}
