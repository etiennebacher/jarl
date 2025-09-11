suppressPackageStartupMessages({
  library(data.table)
  library(jsonlite)
})

all_files <- list.files(
  "results",
  pattern = "\\.json$",
  full.names = TRUE
)
all_files_name <- basename(all_files)

all_repos <- sub("^([^_]+)_([^_]+)_.*\\.json$", "\\1/\\2", all_files_name) |>
  unique()


cat("### Ecosystem checks\n\n", file = "lint_comparison.md")

for (repos in all_repos) {
  message("Processing results of ", repos)
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
    data.table(
      name = x$message$name,
      body = x$message$body,
      filename = x$filename,
      row = x$location$row,
      column = x$location$column
    )
  }) |>
    rbindlist()

  pr_results <- lapply(pr_results_json, \(x) {
    data.table(
      name = x$message$name,
      body = x$message$body,
      filename = x$filename,
      row = x$location$row,
      column = x$location$column
    )
  }) |>
    rbindlist()

  if (identical(dim(main_results), c(0L, 0L))) {
    main_results <- data.table(
      name = character(0),
      body = character(0),
      filename = character(0),
      row = integer(0),
      column = integer(0)
    )
  }

  if (identical(dim(pr_results), c(0L, 0L))) {
    pr_results <- data.table(
      name = character(0),
      body = character(0),
      filename = character(0),
      row = integer(0),
      column = integer(0)
    )
  }

  new_lints <- pr_results[!main_results, on = .(name, filename, row, column)]
  deleted_lints <- main_results[
    !pr_results,
    on = .(name, filename, row, column)
  ]

  msg_header <- paste0(
    "<details><summary><a href=\"https://github.com/",
    repos,
    "\">",
    repos,
    "</a>: +",
    nrow(new_lints),
    " -",
    nrow(deleted_lints),
    " violations</summary>\n\n"
  )

  msg_new_violations <- if (nrow(new_lints) > 0) {
    paste(
      c(
        "<br>\nNew violations:<pre>",
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
          "\n",
          collapse = "\n"
        )
      ),
      collapse = ""
    )
  } else {
    ""
  }
  msg_old_violations <- if (nrow(deleted_lints) > 0) {
    paste(
      c(
        "<br>\nViolations removed:<pre>",
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
          "\n",
          collapse = "\n"
        )
      ),
      collapse = ""
    )
  } else {
    ""
  }

  msg_bottom <- "</pre></details>\n\n"

  paste(
    msg_header,
    msg_new_violations,
    msg_old_violations,
    msg_bottom,
    collapse = ""
  ) |>
    cat(file = "lint_comparison.md", append = TRUE)
}
