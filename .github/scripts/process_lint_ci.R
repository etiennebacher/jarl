suppressPackageStartupMessages({
  library(data.table)
  library(jsonlite)
})

all_files <- list.files("results", pattern = "\\.json$", full.names = TRUE)
all_files_name <- basename(all_files)

all_repos <- sub("^([^_]+)_([^_]+)_.*\\.json$", "\\1/\\2", all_files_name) |>
  unique()

for (repos in all_repos) {
  main_results_json <- jsonlite::read_json(paste0(
    "results/",
    gsub("/", "_", repos),
    "_main.json"
  ))
  pr_results_json <- jsonlite::read_json(paste0(
    "results/",
    gsub("/", "_", repos),
    "_pr.json"
  ))

  main_results <- lapply(main_results_json, \(x) {
    data.frame(
      name = x$message$name,
      body = x$message$body,
      filename = x$filename,
      row = x$location$row,
      column = x$location$column
    )
  }) |>
    rbindlist()

  pr_results <- lapply(pr_results_json, \(x) {
    data.frame(
      name = x$message$name,
      body = x$message$body,
      filename = x$filename,
      row = x$location$row,
      column = x$location$column
    )
  }) |>
    rbindlist()

  new_lints <- pr_results[!main_results, on = .(name, filename, row, column)]
  deleted_lints <- main_results[
    !pr_results,
    on = .(name, filename, row, column)
  ]

  paste0(
    "<details><summary><a href=\"https://github.com/",
    repos,
    "\">",
    repos,
    "</a>: +",
    nrow(new_lints),
    " -",
    nrow(deleted_lints),
    " violations</summary>\n\n",

    if (nrow(new_lints) > 0) {
      c(
        "\n\nNew violations:<pre>\n",
        paste0(
          new_lints$filename,
          "[",
          new_lints$row,
          ":",
          new_lints$column,
          "]: ",
          new_lints$name,
          " -- ",
          new_lints$body,
          "\n"
        )
      )
    },
    if (nrow(deleted_lints) > 0) {
      c(
        "\n\nViolations removed:<pre>\n",
        paste0(
          deleted_lints$filename,
          "[",
          deleted_lints$row,
          ":",
          deleted_lints$column,
          "]: ",
          deleted_lints$name,
          " -- ",
          deleted_lints$body,
          "\n"
        )
      )
    },
    "</pre></details>\n\n"
  ) |>
    cat(file = "lint_comparison.md", append = TRUE)
}
