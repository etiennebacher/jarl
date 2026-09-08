use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{get_function_name, line_start, next_line_start, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstNodeList, TextRange, TextSize};

/// A top-level statement that is either a `library()` call (possibly wrapped
/// in `suppressMessages()`/`suppressPackageStartupMessages()`) or an `if`
/// statement whose branches only contain such calls.
struct LibraryStatement {
    node: RSyntaxNode,
    range: TextRange,
}

/// Version added: 0.6.0
///
/// ## What it does
///
/// Reports `library()` calls that are not grouped at the top of the script,
/// and moves them there.
///
/// A preamble of setup code is allowed before the first `library()` call;
/// what matters is that every `library()` call in the script forms a single
/// consecutive block starting at the first one.
///
/// Only `library()` is considered: `require()` returns a value that is
/// routinely used for its result, so it is out of scope for this rule.
///
/// This rule is skipped in R Markdown and Quarto documents, where it is often
/// more acceptable to have `library()` calls in various chunks.
///
/// ## Why is this bad?
///
/// Scripts where `library()` calls are scattered between the code are hard to
/// read: a reader cannot tell at a glance which packages the script needs,
/// and the attach order (which decides masking) becomes accidental rather
/// than deliberate.
///
/// This rule has an unsafe fix: moving `library()` calls changes attach order
/// and therefore which package masks which.
///
/// This rule is disabled by default.
///
/// ## Example
///
/// ```r
/// library(dplyr)
/// x <- 1
/// library(purrr)
/// ```
///
/// Use instead:
/// ```r
/// library(dplyr)
/// library(purrr)
/// x <- 1
/// ```
pub fn library_call(expressions: &[RSyntaxNode], contents: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let statements: Vec<Option<LibraryStatement>> =
        expressions.iter().map(classify_library_statement).collect();

    let Some(first) = statements.iter().position(Option::is_some) else {
        return diagnostics;
    };

    let mut block_end = first;
    while block_end + 1 < statements.len() && statements[block_end + 1].is_some() {
        block_end += 1;
    }

    let block_end_range = statements[block_end]
        .as_ref()
        .expect("block_end is a library statement")
        .range;
    let insertion_point =
        TextSize::from(next_line_start(contents, usize::from(block_end_range.end())) as u32);

    for stmt in statements.iter().skip(block_end + 1).flatten() {
        diagnostics.push(misplaced_diagnostic(stmt, contents, insertion_point));
    }

    diagnostics
}

/// Classify a top-level statement as a library statement, if it is one.
fn classify_library_statement(stmt: &RSyntaxNode) -> Option<LibraryStatement> {
    if let Some(call) = RCall::cast(stmt.clone()) {
        if !is_library_call(&call) {
            return None;
        }
        return Some(LibraryStatement {
            node: stmt.clone(),
            range: stmt.text_trimmed_range(),
        });
    }

    if let Some(if_stmt) = RIfStatement::cast(stmt.clone())
        && if_is_library_only(&if_stmt)
    {
        return Some(LibraryStatement {
            node: stmt.clone(),
            range: stmt.text_trimmed_range(),
        });
    }

    None
}

/// Whether `call` is a `library()` call, directly or wrapped in
/// `suppressMessages()`/`suppressPackageStartupMessages()`.
fn is_library_call(call: &RCall) -> bool {
    let Some(name) = call.function().ok().map(get_function_name) else {
        return false;
    };

    if name == "library" {
        return true;
    }

    if name != "suppressMessages" && name != "suppressPackageStartupMessages" {
        return false;
    }

    let Some(mut items) = call.arguments().ok().map(|args| args.items().into_iter()) else {
        return false;
    };
    let Some(Ok(only)) = items.next() else {
        return false;
    };
    if items.next().is_some() {
        return false;
    }

    let Some(inner_call) = only.value().and_then(|value| value.as_r_call().cloned()) else {
        return false;
    };
    inner_call.function().ok().map(get_function_name).as_deref() == Some("library")
}

/// Whether every branch of `if_stmt` contains only library statements.
fn if_is_library_only(if_stmt: &RIfStatement) -> bool {
    let RIfStatementFields { consequence, else_clause, .. } = if_stmt.as_fields();
    let Ok(consequence) = consequence else {
        return false;
    };
    if !branch_is_library_only(&consequence) {
        return false;
    }

    match else_clause {
        None => true,
        Some(clause) => match clause.alternative() {
            Ok(alternative) => branch_is_library_only(&alternative),
            Err(_) => false,
        },
    }
}

/// A branch is library-only when it's a braced block whose statements are all
/// library statements, or a single library statement itself (including a
/// nested `if` for `else if` chains).
fn branch_is_library_only(branch: &AnyRExpression) -> bool {
    if let Some(braced) = branch.as_r_braced_expressions() {
        return braced
            .expressions()
            .iter()
            .all(|expr| classify_library_statement(expr.syntax()).is_some());
    }
    classify_library_statement(branch.syntax()).is_some()
}

/// Whether `range` is alone on its lines: nothing but whitespace before it on
/// its first line, and nothing but whitespace after it on its last line.
fn statement_alone_on_lines(contents: &str, range: TextRange) -> bool {
    let start = usize::from(range.start());
    let end = usize::from(range.end());
    let before = &contents[line_start(contents, start)..start];
    let after = &contents[end..next_line_start(contents, end)];
    before.trim().is_empty() && after.trim().is_empty()
}

fn misplaced_diagnostic(
    stmt: &LibraryStatement,
    contents: &str,
    insertion_point: TextSize,
) -> Diagnostic {
    let start = usize::from(stmt.range.start());
    let end = usize::from(stmt.range.end());
    let text = contents[start..end].to_string();

    let to_skip =
        node_contains_comments(&stmt.node) || !statement_alone_on_lines(contents, stmt.range);

    let deletion =
        Edit::deletion_with_offsets(line_start(contents, start), next_line_start(contents, end));
    let insertion = Edit::insertion(insertion_point, format!("{text}\n"));

    Diagnostic::new(
        ViolationData::new(
            Rule::LibraryCall,
            "`library()` calls should be grouped at the top of the script.".to_string(),
            Some("Move this call next to the other `library()` calls.".to_string()),
        ),
        stmt.range,
        Fix::from_edits(vec![deletion, insertion], to_skip),
    )
}
