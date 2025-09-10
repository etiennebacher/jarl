suppressPackageStartupMessages({
  library(data.table)
  library(jsonlite)
  library(poorman)
})

main_results_json <- jsonlite::read_json("results_main.json")
branch_results_json <- jsonlite::read_json("results_pr.json")

main_results <- lapply(main_results_json, \(x) {
  data.frame(
    name = x$message$name,
    filename = x$filename,
    row = x$location$row,
    column = x$location$column
  )
}) |>
  rbindlist()

branch_results <- lapply(branch_results_json, \(x) {
  data.frame(
    name = x$message$name,
    filename = x$filename,
    row = x$location$row,
    column = x$location$column
  )
}) |>
  rbindlist()

new_lints <- anti_join(
  branch_results,
  main_results,
  by = c("name", "filename", "row", "column")
) |>
  nrow()

deleted_lints <- anti_join(
  main_results,
  branch_results,
  by = c("name", "filename", "row", "column")
) |>
  nrow()

paste0("**dplyr**: +", new_lints, " -", deleted_lints, " violations") |>
  writeLines("lint_comparison.md")
