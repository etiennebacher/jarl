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

repos_raw <-
  "pola-rs/r-polars@055f891d977d6a8004fe8ab1bb5d47cfdd44872a
Rdatatable/data.table@d7833cb3a0c36ab329fcfe5cc1523449663f898f
ropensci/targets@d45243e1a239da467c6e96e0069ecb7368f78021
rstudio/shiny@b6e9e9d216cd574cf8985d23772399f51ee4a4ac
tidyverse/dplyr@384ff2c4af79a9bd9142d8bdab23efbcd52f275b
tidyverse/ggplot2@bed8e32d45c7dc98afb783b409edac98854d41a7
vincentarelbundock/marginaleffects@ffe5a4f5962a9ba51c3e68e1c55eea62887c25da
wch/r-source@b5cf23a805f4305852f55c4148f2a302b06844b5"
repo_lines <- strsplit(repos_raw, "\n")[[1]]
repo_lines <- repo_lines[repo_lines != ""]
repo_parts <- strsplit(repo_lines, "@")
all_repos <- setNames(
  lapply(repo_parts, function(x) trimws(x[2])), # the commit SHAs
  sapply(repo_parts, function(x) trimws(x[1])) # the repo names
)

cat("### Ecosystem checks\n\n", file = "lint_comparison.md")

for (i in seq_along(all_repos)) {
  repos <- names(all_repos)[i]
  repos_sha <- all_repos[[i]]

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
    "/tree/",
    repos_sha,
    "\">",
    repos,
    "</a>: +",
    nrow(new_lints),
    " -",
    nrow(deleted_lints),
    " violations</summary>\n\n"
  )

  msg_new_violations <- if (nrow(new_lints) > 0) {
    new_lints <- head(new_lints, 100)
    paste(
      c(
        "<br>\nNew violations (first 100):<pre>",
        paste0(
          "<a href=\"https://github.com/",
          repos,
          "/tree/",
          repos_sha,
          "/",
          new_lints$filename,
          "#L",
          new_lints$row,
          "\">",
          new_lints$filename,
          "[",
          new_lints$row,
          ":",
          new_lints$column,
          "]",
          "</a>: ",
          new_lints$name,
          " -- ",
          new_lints$body,
          collapse = "\n"
        )
      ),
      collapse = ""
    )
  } else {
    ""
  }
  msg_old_violations <- if (nrow(deleted_lints) > 0) {
    deleted_lints <- head(deleted_lints, 100)
    paste(
      c(
        "<br>\nViolations removed (first 100):<pre>",
        paste0(
          "<a href=\"https://github.com/",
          repos,
          "/tree/",
          repos_sha,
          "/",
          deleted_lints$filename,
          "#L",
          deleted_lints$row,
          "\">",
          deleted_lints$filename,
          "[",
          deleted_lints$row,
          ":",
          deleted_lints$column,
          "]",
          "</a>: ",
          deleted_lints$name,
          " -- ",
          deleted_lints$body,
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
