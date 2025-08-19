use crate::location::Location;
use crate::message::Diagnostic;
use air_r_syntax::{
    AnyRExpression, RArgument, RArgumentList, RCall, RCallFields, RExtractExpressionFields,
    RSyntaxKind, RSyntaxNode,
};
use anyhow::{Result, anyhow};
use biome_rowan::AstNode;
use biome_rowan::AstSeparatedList;

pub fn find_new_lines(ast: &RSyntaxNode) -> Result<Vec<usize>> {
    match ast.first_child() {
        Some(rootnode) => Ok(rootnode
            .text()
            .to_string()
            .match_indices("\n")
            .map(|x| x.0)
            .collect::<Vec<usize>>()),
        None => Err(anyhow!(
            "Couldn't find root node. Maybe the document contains a parsing error?"
        )),
    }
}

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

    let col: usize = start - last_new_line;
    let row: usize = n_new_lines + 1;
    (row, col)
}

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

pub fn get_arg_by_position(args: &RArgumentList, pos: usize) -> Option<RArgument> {
    args.iter().nth(pos - 1).map(|x| x.unwrap())
}

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
            if let Some(name_clause) = arg.name_clause() {
                if let Ok(n) = name_clause.name() {
                    if n.to_string().trim() == name {
                        dropped_by_name = true;
                        return None;
                    }
                }
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

pub fn is_argument_present(args: &RArgumentList, name: &str, position: usize) -> bool {
    get_arg_by_name_then_position(args, name, position).is_some()
}

pub fn get_first_arg(node: &RSyntaxNode) -> Option<RSyntaxNode> {
    node.descendants()
        .find(|x| x.kind() == RSyntaxKind::R_ARGUMENT)
}

pub fn get_args(node: &RSyntaxNode) -> Vec<RSyntaxNode> {
    node.descendants()
        // Limit to first list of arguments to avoid collecting arguments from nested functions
        .find(|x| x.kind() == RSyntaxKind::R_ARGUMENT_LIST)
        .unwrap()
        .descendants()
        .filter(|x| x.kind() == RSyntaxKind::R_ARGUMENT)
        .collect::<Vec<_>>()
}

pub fn node_is_in_square_brackets(ast: &RSyntaxNode) -> bool {
    let great_grandparent = ast.ancestors().nth(3);
    match great_grandparent {
        Some(x) => x.kind() == RSyntaxKind::R_SUBSET_ARGUMENTS,
        None => false,
    }
}

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

// Takes an RCall.
// If this RCall corresponds to a nested function of the form
// `outer_fn(inner_fn(content))`, then it returns `content`, otherwise None.
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

        let inner_content = arguments?.items().into_syntax().text().to_string();

        Ok(Some(inner_content))
    } else {
        Ok(None)
    }
}

// #[cfg(test)]
// mod tests {

//     use std::fs;
//     use std::process::{Command, Stdio};
//     use tempfile::Builder;

//     #[test]
//     fn parsing_error_doesnt_panic() {
//         let temp_file = Builder::new()
//             .prefix("test-flir")
//             .suffix(".R")
//             .tempfile()
//             .unwrap();

//         fs::write(&temp_file, "blah = fun(1) {").expect("Failed to write initial content");

//         let output = Command::new("flir")
//             .arg(temp_file.path())
//             .stdout(Stdio::piped())
//             .output()
//             .expect("Failed to execute command");

//         let err_message = String::from_utf8_lossy(&output.stderr).to_string();
//         println!("{err_message}");
//         assert!(err_message.contains("Maybe the document contains a parsing error"))
//     }
// }
