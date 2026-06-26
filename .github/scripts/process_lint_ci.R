suppressPackageStartupMessages({
  library(data.table)
  library(jsonlite)
})

all_files <- list.files(
  "results",
  pattern = "\\.json$",
  full.names = TRUE
)

repos_raw <- Sys.getenv("TEST_REPOS")
repo_lines <- strsplit(repos_raw, "\n")[[1]]
repo_lines <- trimws(repo_lines)
repo_lines <- repo_lines[repo_lines != ""]

# Parse the repo list. Comment lines (e.g. "# packages", "# other") act as
# category markers: every repo listed below such a line belongs to that
# category, which is later used to group results under a subheader.
repo_names <- character(0)
repo_shas <- character(0)
repo_categories <- character(0)
current_category <- NA_character_

for (line in repo_lines) {
  if (startsWith(line, "#")) {
    marker <- trimws(sub("^#+", "", line))
    current_category <- if (grepl("package", marker, ignore.case = TRUE)) {
      "Packages"
    } else {
      "Other repos"
    }
    next
  }
  parts <- strsplit(line, "@")[[1]]
  repo_names <- c(repo_names, trimws(parts[1]))
  repo_shas <- c(repo_shas, trimws(parts[2]))
  repo_categories <- c(repo_categories, current_category)
}

total_new <- 0
total_deleted <- 0
body <- character(0)
last_printed_category <- NULL

for (i in seq_along(repo_names)) {
  repos <- repo_names[i]
  repos_sha <- repo_shas[i]
  category <- repo_categories[i]

  message("Processing results of ", repos)
  main_results_json <- jsonlite::read_json(paste0(
    "results/",
    gsub("/", "_", repos),
    "_main.json"
  ))[["diagnostics"]]
  pr_results_json <- jsonlite::read_json(paste0(
    "results/",
    gsub("/", "_", repos),
    "_pr.json"
  ))[["diagnostics"]]

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

  if (nrow(new_lints) == 0 && nrow(deleted_lints) == 0) {
    next
  }

  total_new <- total_new + nrow(new_lints)
  total_deleted <- total_deleted + nrow(deleted_lints)

  # Add a category subheader the first time a repo with changes shows up in
  # that category.
  if (!is.na(category) && !identical(last_printed_category, category)) {
    body <- c(body, paste0("## ", category, "\n\n"))
    last_printed_category <- category
  }

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
    new_lints <- head(new_lints, 50)
    paste(
      c(
        "<br>\nNew violations (first 50):<pre>",
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
    deleted_lints <- head(deleted_lints, 50)
    paste(
      c(
        "<br>\nViolations removed (first 50):<pre>",
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

  body <- c(
    body,
    paste(
      msg_header,
      msg_new_violations,
      msg_old_violations,
      msg_bottom,
      collapse = ""
    )
  )
}

cat("# Ecosystem results\n\n", file = "lint_comparison.md")

if (length(body) == 0) {
  cat(
    "✅ No new or removed violations\n",
    file = "lint_comparison.md",
    append = TRUE
  )
} else {
  cat(
    paste0(
      total_new,
      " violations added, ",
      total_deleted,
      " violations removed\n\n"
    ),
    file = "lint_comparison.md",
    append = TRUE
  )
  cat(paste(body, collapse = ""), file = "lint_comparison.md", append = TRUE)
}
