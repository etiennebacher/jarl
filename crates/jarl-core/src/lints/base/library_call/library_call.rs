use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{get_function_name, line_start, next_line_start};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstNodeList, TextRange, TextSize};

/// A top-level statement that is either a `library()` call (possibly wrapped
/// in `suppressMessages()`/`suppressPackageStartupMessages()`) or an `if`
/// statement whose branches only contain such calls.
struct LibraryStatement {
    range: TextRange,
}

/// <!-- docs: start -->
/// Version added: 0.7.0
///
/// ## What it does
///
/// Reports `library()` calls that are not grouped at the top of the script.
///
/// ## Why is this bad?
///
/// Scripts where `library()` calls are scattered between the code are hard to
/// read: a reader cannot tell at a glance which packages the script needs. This
/// rule has several special cases:
///
/// - an `if` statement that only contains `library()` calls is considered
///   equivalent to a simple `library()` call;
/// - similarly, `suppressMessages()` and `suppressPackageStartupMessages()`
///   containing `library()` calls are considered equivalent to a simple
///   `library()` call;
/// - `options()` and `Sys.setenv()` at the top of the script stay there. Jarl
///   will not move `library()` calls above these functions.
/// - this rule is skipped in R Markdown and Quarto documents, where it is more
///   common to have `library()` calls in various chunks.
///
/// Comments located on the same line as the `library()` call are moved with it,
/// but comments preceding it are not.
///
/// This rule has an unsafe fix that moves `library()` calls towards the top.
/// This is unsafe because code that would initially run before some `library()`
/// calls would run after and therefore could be affected by new namespace
/// conflicts.
///
/// This rule is disabled by default.
///
/// ## Example
///
/// ```r
/// library(dplyr)
/// x <- 1
/// library(purrr)
/// y <- 2
/// library(data.table)
/// ```
///
/// Use instead:
/// ```r
/// library(dplyr)
/// library(purrr)
/// library(data.table)
/// x <- 1
/// y <- 2
/// ```
/// <!-- docs: end -->
pub fn library_call(expressions: &[RSyntaxNode], contents: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let statements: Vec<Option<LibraryStatement>> =
        expressions.iter().map(classify_library_statement).collect();

    // The library block may only be preceded by `options()`/`Sys.setenv()`
    // calls. Misplaced calls are moved right after the block, or at the very
    // top of the file (before any comment) if there is no such block.
    let preamble_len = expressions
        .iter()
        .take_while(|expr| is_setup_call(expr))
        .count();
    let block_len = statements[preamble_len..]
        .iter()
        .take_while(|s| s.is_some())
        .count();
    let insertion_point = match preamble_len + block_len {
        0 => 0,
        n => next_line_start(
            contents,
            usize::from(expressions[n - 1].text_trimmed_range().end()),
        ),
    };
    let insertion_point = TextSize::from(insertion_point as u32);

    for stmt in statements.iter().skip(preamble_len + block_len).flatten() {
        diagnostics.push(misplaced_diagnostic(stmt, contents, insertion_point));
    }

    diagnostics
}

/// Whether `stmt` is an `options()` or `Sys.setenv()` call.
fn is_setup_call(stmt: &RSyntaxNode) -> bool {
    RCall::cast(stmt.clone())
        .and_then(|call| call.function().ok())
        .map(get_function_name)
        .is_some_and(|name| name == "options" || name == "Sys.setenv")
}

/// Classify a top-level statement as a library statement, if it is one.
fn classify_library_statement(stmt: &RSyntaxNode) -> Option<LibraryStatement> {
    if let Some(call) = RCall::cast(stmt.clone()) {
        if !is_library_call(&call) {
            return None;
        }
        return Some(LibraryStatement { range: stmt.text_trimmed_range() });
    }

    if let Some(if_stmt) = RIfStatement::cast(stmt.clone())
        && if_is_library_only(&if_stmt)
    {
        return Some(LibraryStatement { range: stmt.text_trimmed_range() });
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
/// its first line, and nothing but whitespace or a comment after it on its
/// last line.
fn statement_alone_on_lines(contents: &str, range: TextRange) -> bool {
    let start = usize::from(range.start());
    let end = usize::from(range.end());
    let before = &contents[line_start(contents, start)..start];
    let after = contents[end..next_line_start(contents, end)].trim();
    before.trim().is_empty() && (after.is_empty() || after.starts_with('#'))
}

fn misplaced_diagnostic(
    stmt: &LibraryStatement,
    contents: &str,
    insertion_point: TextSize,
) -> Diagnostic {
    let start = usize::from(stmt.range.start());
    let end = next_line_start(contents, usize::from(stmt.range.end()));
    // Include the trailing comment on the last line, if any.
    let text = contents[start..end].trim_end();

    let to_skip = !statement_alone_on_lines(contents, stmt.range);

    let deletion = Edit::deletion_with_offsets(line_start(contents, start), end);
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
