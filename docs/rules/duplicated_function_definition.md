# duplicated_function_definition
## What it does

Checks for duplicated function definitions in R packages.

## Why is this bad?

Having two functions with the same name is likely an error since development
tools such as `devtools::load_all()` will only load one of them. This rule
looks for function definitions shared across files in the same R package,
meaning files that are in a folder named "R" whose parent folder has a
`DESCRIPTION` file.

This rule doesn't have an automatic fix.

## Example

```r
# In "R/foo1.R":
foo <- function(x) {
  x + 1
}

# In "R/foo2.R":
foo <- function(x) {
  x + 2
}

# Function "foo" is defined in two different scripts in the same package,
# which is likely due to a mistake.
```

