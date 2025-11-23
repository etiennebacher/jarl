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

repos_raw <- Sys.getenv("TEST_REPOS")
repo_lines <- strsplit(repos_raw, "\n")[[1]]
repo_lines <- repo_lines[repo_lines != ""]
repo_parts <- strsplit(repo_lines, "@")
all_repos <- setNames(
  lapply(repo_parts, function(x) trimws(x[2])), # the commit SHAs
  sapply(repo_parts, function(x) trimws(x[1])) # the repo names
)

cat("### Ecosystem checks\n\n", file = "lint_comparison.md")

n_without_changes <- 0

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

  # if (nrow(new_lints) == 0 && nrow(deleted_lints) == 0) {
  #   n_without_changes <- n_without_changes + 1

  #   # If we are at the last repo and there were no changes anywhere, return
  #   # early. Otherwise keep going.
  #   if (n_without_changes == length(all_repos)) {
  #     cat(
  #       "✅ No new or removed violations",
  #       file = "lint_comparison.md",
  #       append = TRUE
  #     )
  #     break
  #   } else {
  #     next
  #   }
  # }

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
