# object_name
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

This rule checks the names of variables and arguments used in assignments
and function definitions.
For `$` and `@` assignments, it checks only the base object name.

This rule is disabled by default.

The default styles are `snake_case` and `symbols`:

## Why is this bad?

Consistent names make code easier to read and maintain.

## Example

```r
badName <- 1
f <- function(badArg) {
  badArg
}
badName$member <- 1
```

Use instead:

```r
bad_name <- 1
f <- function(bad_arg) {
  bad_arg
}
bad_name$member <- 1
```

## Configuration

```toml
[lint.object_name]
styles = ["snake_case", "symbols"]
```

Built-in styles are `CamelCase`, `camelCase`, `snake_case`, `SNAKE_CASE`,
`dotted.case`, `lowercase`, `UPPERCASE`, and `symbols`.
Any combination of default styles can be included.

The following special names are exempt from style checks by default:
`.onLoad`, `.onAttach`, `.onUnload`, `.onDetach`, `.Last.lib`, `.First`, and
`.Last`.
Use `extend-special-names` to add project-specific special names while keeping
the defaults:

```toml
[lint.object_name]
extend-special-names = ["my_special_hook"]
```

Use `special-names` to replace the default set entirely. Do not specify both
`special-names` and `extend-special-names`.

Additional acceptable names can be added via `regexes` when `styles` is set:

```toml
[lint.object_name.regexes]
prefixed = "^x_[a-z]+$"
```

If `styles` is omitted while `regexes` is supplied, the regular expressions are
used to define the accepted styles.
