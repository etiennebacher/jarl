## Overview

All rules belong to at least one category:

-   CORR: correctness, code that is outright wrong or useless.
-   SUSP: suspicious, code that is most likely wrong or useless.
-   PERF: performance, code that can be written to run faster.
-   READ: readability, code is correct but can be written in a way that is easier to read.

You can find the list of available rules below, and more detailed explanations and examples in pages in the sidebar.

::: {.callout-note}
## Comparison to `lintr`

`lintr` includes many rules related to code formatting, such as [`brace_linter`](https://lintr.r-lib.org/dev/reference/brace_linter.html), [`line_length_linter`](https://lintr.r-lib.org/dev/reference/line_length_linter.html), and [`paren_body_linter`](https://lintr.r-lib.org/dev/reference/paren_body_linter.html), among others.

**Supporting those rules is not an objective of Jarl.**
Instead, I recommend using the [Air formatter](https://posit-dev.github.io/air/).
:::

The column "Has fix?" can take the following values:

-   ✅ safe fix

-   ❗Unsafe fix

-   ❌ No fix

| Rule name            | Group      | Has fix? |
|----------------------|------------|----------|
| all_equal            | SUSP       | ❗       |
| any_duplicated       | PERF       | ✅       |
| any_is_na            | PERF       | ✅       |
| assignment           | READ       | ✅       |
| class_equals         | SUSP       | ❗       |
| coalesce             | READ       | ✅       |
| duplicated_arguments | SUSP       | ❌       |
| empty_assignment     | READ       | ❌       |
| equals_na            | CORR       | ✅       |
| for_loop_index       | READ       | ❌       |
| grepv                | READ       | ✅       |
| implicit_assignment  | READ       | ❌       |
| is_numeric           | READ       | ✅       |
| length_levels        | READ       | ✅       |
| length_test          | CORR       | ✅       |
| lengths              | PERF, READ | ✅       |
| matrix_apply         | PERF       | ✅       |
| numeric_leading_zero | READ       | ✅       |
| redundant_equals     | READ       | ✅       |
| repeat               | READ       | ✅       |
| sample_int           | READ       | ✅       |
| sort                 | PERF, READ | ✅       |
| true_false_symbol    | READ       | ❌       |
| which_grepl          | PERF, READ | ✅       |
