# unused_function
// ## What it does
//
// Checks for unused functions, currently limited to R packages. It looks for
// functions defined in the `R` folder that are not exported and not used
// anywhere in the package (including the `R`, `inst/tinytest`, `src`, and
// `tests` folders).
//
// ## Why is this bad?
//
// An internal function that is never called is likely dead code left over from
// refactoring. Removing it keeps the codebase easier to understand and
// maintain.
//
// ## Limitations
//
// There are many ways to call a function in R code (e.g. `foo()`,
// `do.call("foo", ...)`, `lapply(x, foo)` among others). Jarl tries to limit
// false positives as much as possible, at the expense of false negatives. This
// means that reporting a function that is actually used somewhere (false positive)
// is considered a bug, but not reporting a function that isn't used anywhere
// (false negative) isn't considered a bug (but can be suggested as a feature
// request).
//
// ## Example
//
// ```r
// # In NAMESPACE: export(public_fn)
//
// # In R/public.R:
// public_fn <- function(x) {
//   check_character(x)
// }
//
// # In R/helper.R:
// check_character <- function(x) {
//   stopifnot(is.character(x))
// }
// check_length <- function(x, y) {
//   stopifnot(length(x) == y)
// }
//
// # `public_fn` is exported by the package, so it is considered used.
// # `check_character()` isn't exported but used in `public_fn`.
// # `check_length()` isn't exported but and isn't used anywhere, so it is
// # reported.
// ```

Find a NAMESPACE directive (e.g. `S3method`, `export`) in a line and
return its parenthesized arguments. Handles lines where the directive is
preceded by an `if (...)` guard, e.g.:
  `if (getRversion() >= "4.4.0") S3method(sort_by, data.table)`
fn extract_directive<'a>(line: &'a str, directive: &str) -> Option<&'a str> {
    // Find `directive(` in the line
    let dir_with_paren = format!("{directive}(");
    let start = line.find(&dir_with_paren)?;
    let args_start = start + dir_with_paren.len();

    // Make sure the directive is not part of a longer word
    // (e.g. "exportPattern" should not match "export")
    if start > 0 {
        let prev = line.as_bytes()[start - 1];
        if prev.is_ascii_alphanumeric() || prev == b'_' {
            return None;
        }
    }

    // Find the matching closing paren
    let rest = &line[args_start..];
    let end = rest.rfind(')')?;
    Some(&rest[..end])
}

Parse a NAMESPACE file and return the set of exported function names.

Handles both `export(name)` directives and `exportPattern(regex)` directives.
For `exportPattern`, the regex is compiled and matched against `all_names`
to expand it into concrete names.
