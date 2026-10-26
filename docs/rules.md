---
title: Rules
---

All rules belong to at least one category:

-   **comments** (COMM): mistakes in the suppression comments (`# jarl-ignore`).
-   **correctness** (CORR): code that is outright wrong or useless.
-   **dplyr** (DPLYR): rules for the package `dplyr`. Disabled by default.
-   **performance** (PERF): code that can be written to run faster.
-   **readability** (READ): code is correct but can be written in a way that is easier to read.
-   **suspicious** (SUSP): code that is most likely wrong or useless.
-   **testthat** (TESTTHAT): rules for the package `testthat`. Disabled by default.

You can find the list of available rules below, and more detailed explanations and examples in pages in the sidebar.

!!! note "Comparison to `lintr`"

    `lintr` includes many rules related to code formatting, such as [`brace_linter`](https://lintr.r-lib.org/dev/reference/brace_linter.html), [`line_length_linter`](https://lintr.r-lib.org/dev/reference/line_length_linter.html), and [`paren_body_linter`](https://lintr.r-lib.org/dev/reference/paren_body_linter.html), among others.

    **Supporting those rules is not an objective of Jarl.**
    Instead, I recommend using the [Air formatter](https://posit-dev.github.io/air/).

The column "Has fix" can take the following values:

-   ✅ Safe fix

-   ❗ Unsafe fix

-   ❌ No fix

<div id="rules-table" markdown>

| Rule name | Group | Has fix | Conditions |
|-----------|-------|---------|------------|
| [all_equal](rules/all_equal.md) | suspicious | ❗ |  |
| [any_duplicated](rules/any_duplicated.md) | performance | ✅ |  |
| [any_is_na](rules/any_is_na.md) | performance | ✅ |  |
| [assignment](rules/assignment.md) | readability | ✅ | Disabled by default |
| [assignment_on_if_no_else](rules/assignment_on_if_no_else.md) | suspicious | ❌ |  |
| [blanket_suppression](rules/blanket_suppression.md) | comments | ❌ |  |
| [class_equals](rules/class_equals.md) | suspicious | ❗ |  |
| [coalesce](rules/coalesce.md) | readability | ✅ | R >= 4.4 |
| [comparison_negation](rules/comparison_negation.md) | readability | ❗ |  |
| [condition_call](rules/condition_call.md) | readability | ❗ | Disabled by default |
| [condition_message](rules/condition_message.md) | readability | ✅ | Disabled by default |
| [consecutive_signs](rules/consecutive_signs.md) | readability, suspicious | ❌ |  |
| [cyclomatic_complexity](rules/cyclomatic_complexity.md) | readability | ❌ | Disabled by default |
| [dplyr_filter_out](rules/dplyr_filter_out.md) | dplyr | ✅ | Disabled by default |
| [dplyr_group_by_ungroup](rules/dplyr_group_by_ungroup.md) | dplyr | ✅ | Disabled by default |
| [download_file](rules/download_file.md) | suspicious | ❌ |  |
| [duplicated_arguments](rules/duplicated_arguments.md) | suspicious | ❌ |  |
| [duplicated_function_definition](rules/duplicated_function_definition.md) | correctness | ❌ |  |
| [empty_assignment](rules/empty_assignment.md) | readability | ❌ |  |
| [empty_file](rules/empty_file.md) | suspicious | ❌ |  |
| [equals_na](rules/equals_na.md) | correctness | ✅ |  |
| [equals_nan](rules/equals_nan.md) | correctness | ✅ |  |
| [equals_null](rules/equals_null.md) | correctness | ✅ |  |
| [expect_length](rules/expect_length.md) | testthat | ✅ | Disabled by default |
| [expect_match](rules/expect_match.md) | testthat | ✅ | Disabled by default |
| [expect_named](rules/expect_named.md) | testthat | ✅ | Disabled by default |
| [expect_no_match](rules/expect_no_match.md) | testthat | ✅ | Disabled by default |
| [expect_not](rules/expect_not.md) | testthat | ✅ | Disabled by default |
| [expect_null](rules/expect_null.md) | testthat | ✅ | Disabled by default |
| [expect_s3_class](rules/expect_s3_class.md) | testthat | ✅ | Disabled by default |
| [expect_s4_class](rules/expect_s4_class.md) | testthat | ✅ | Disabled by default |
| [expect_true_false](rules/expect_true_false.md) | testthat | ✅ | Disabled by default |
| [expect_type](rules/expect_type.md) | testthat | ✅ | Disabled by default |
| [fixed_regex](rules/fixed_regex.md) | performance | ✅ | Disabled by default |
| [for_loop_dup_index](rules/for_loop_dup_index.md) | correctness, suspicious | ❌ |  |
| [for_loop_index](rules/for_loop_index.md) | readability | ❌ |  |
| [glue](rules/glue.md) | correctness | ❌ |  |
| [grepv](rules/grepv.md) | readability | ✅ | R >= 4.5 |
| [if_always_true](rules/if_always_true.md) | readability, suspicious | ❌ |  |
| [if_not_else](rules/if_not_else.md) | readability | ❌ | Disabled by default |
| [implicit_assignment](rules/implicit_assignment.md) | readability | ❌ |  |
| [internal_function](rules/internal_function.md) | suspicious | ❌ | Disabled by default |
| [invalid_chunk_suppression](rules/invalid_chunk_suppression.md) | comments | ❌ |  |
| [is_numeric](rules/is_numeric.md) | readability | ✅ |  |
| [length_levels](rules/length_levels.md) | readability | ✅ |  |
| [length_test](rules/length_test.md) | correctness | ✅ |  |
| [lengths](rules/lengths.md) | performance, readability | ✅ |  |
| [library_call](rules/library_call.md) | readability | ❗ | Disabled by default |
| [list2df](rules/list2df.md) | performance, readability | ✅ | R >= 4.0 |
| [literal_coercion](rules/literal_coercion.md) | readability | ✅ |  |
| [matrix_apply](rules/matrix_apply.md) | performance | ✅ |  |
| [misnamed_suppression](rules/misnamed_suppression.md) | comments | ❌ |  |
| [misplaced_file_suppression](rules/misplaced_file_suppression.md) | comments | ❌ |  |
| [misplaced_suppression](rules/misplaced_suppression.md) | comments | ❌ |  |
| [missing_argument](rules/missing_argument.md) | suspicious | ❌ |  |
| [nested_pipe](rules/nested_pipe.md) | readability | ❌ | Disabled by default |
| [notin](rules/notin.md) | readability | ✅ | R >= 4.6 |
| [numeric_leading_zero](rules/numeric_leading_zero.md) | readability | ✅ |  |
| [nzchar](rules/nzchar.md) | performance | ❗ | Disabled by default |
| [outdated_suppression](rules/outdated_suppression.md) | comments | ✅ |  |
| [outer_negation](rules/outer_negation.md) | performance, readability | ✅ |  |
| [pipe_return](rules/pipe_return.md) | correctness | ❌ |  |
| [quotes](rules/quotes.md) | readability | ✅ | Disabled by default |
| [redundant_equals](rules/redundant_equals.md) | readability | ✅ |  |
| [rep_times_ignored](rules/rep_times_ignored.md) | suspicious | ❗ |  |
| [repeat](rules/repeat.md) | readability | ✅ |  |
| [sample_int](rules/sample_int.md) | readability | ✅ |  |
| [seq](rules/seq.md) | suspicious | ✅ |  |
| [seq2](rules/seq2.md) | readability, suspicious | ✅ |  |
| [sort](rules/sort.md) | performance, readability | ✅ |  |
| [sprintf](rules/sprintf.md) | correctness, suspicious | ✅ |  |
| [stopifnot_all](rules/stopifnot_all.md) | readability | ❗ | Disabled by default |
| [string_boundary](rules/string_boundary.md) | performance, readability | ❗ |  |
| [strings_as_factors](rules/strings_as_factors.md) | suspicious | ❌ | R < 4.0 |
| [system_file](rules/system_file.md) | readability | ✅ |  |
| [todo_comment](rules/todo_comment.md) | readability | ❌ | Disabled by default |
| [true_false_symbol](rules/true_false_symbol.md) | readability | ❌ |  |
| [undesirable_function](rules/undesirable_function.md) | correctness | ❌ |  |
| [undesirable_operator](rules/undesirable_operator.md) | correctness | ❌ |  |
| [unexplained_suppression](rules/unexplained_suppression.md) | comments | ❌ |  |
| [unmatched_range_suppression](rules/unmatched_range_suppression.md) | comments | ❌ |  |
| [unnecessary_nesting](rules/unnecessary_nesting.md) | readability | ✅ | Disabled by default |
| [unnecessary_parentheses](rules/unnecessary_parentheses.md) | readability | ✅ |  |
| [unreachable_code](rules/unreachable_code.md) | readability, suspicious | ❌ |  |
| [unused_function](rules/unused_function.md) | correctness | ❌ |  |
| [unused_object](rules/unused_object.md) | correctness | ❌ | Disabled by default |
| [vector_logic](rules/vector_logic.md) | performance | ❌ |  |
| [which_grepl](rules/which_grepl.md) | performance, readability | ✅ |  |
| [yoda_condition](rules/yoda_condition.md) | readability | ❗ | Disabled by default |

</div>
