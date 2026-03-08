suppressPackageStartupMessages({
  library(jsonlite)
})

repos_raw <- Sys.getenv("TEST_REPOS")
repo_lines <- strsplit(repos_raw, "\n")[[1]]
repo_lines <- repo_lines[repo_lines != ""]
repo_parts <- strsplit(repo_lines, "@")
all_repos <- setNames(
  lapply(repo_parts, function(x) trimws(x[2])),
  sapply(repo_parts, function(x) trimws(x[1]))
)

cat("### Fix checks\n\n", file = "fix_check.md")

any_failure <- FALSE
parse_error_repos <- character(0)
idempotency_repos <- character(0)

for (i in seq_along(all_repos)) {
  repo <- names(all_repos)[i]
  repo_sha <- all_repos[[i]]
  repo_dir <- gsub("/", "_", repo)

  message("Processing results of ", repo)

  # -- Parse error check -----------------------------------------------------
  # Compare the *sets* of files with parse errors, not just counts. A fix
  # could turn a valid file into one that fails to parse — that file wouldn't
  # appear in the pre-fix error list but would appear in the post-fix list.
  pre_json <- jsonlite::read_json(paste0("results/", repo_dir, "_pre.json"))
  post_json <- jsonlite::read_json(paste0("results/", repo_dir, "_post.json"))

  pre_error_files <- vapply(
    pre_json[["errors"]],
    function(x) x[["file"]],
    character(1)
  )
  post_error_files <- vapply(
    post_json[["errors"]],
    function(x) x[["file"]],
    character(1)
  )
  new_error_files <- setdiff(post_error_files, pre_error_files)

  if (length(new_error_files) > 0) {
    any_failure <- TRUE
    parse_error_repos <- c(parse_error_repos, repo)
    message(
      "  New parse errors in: ",
      paste(new_error_files, collapse = ", ")
    )
  }

  # -- Idempotency check -----------------------------------------------------
  sha1 <- trimws(readLines(
    paste0("results/", repo_dir, "_sha1.txt"),
    warn = FALSE
  ))
  sha2 <- trimws(readLines(
    paste0("results/", repo_dir, "_sha2.txt"),
    warn = FALSE
  ))

  if (sha1 != sha2) {
    any_failure <- TRUE
    idempotency_repos <- c(idempotency_repos, repo)
    message("  Fixes are not idempotent")
  }
}

# -- Generate markdown report ------------------------------------------------

if (!any_failure) {
  cat(
    paste0(
      "All ",
      length(all_repos),
      " repos passed.\n\n",
      "- No fixes introduced new parse errors\n",
      "- All fixes are idempotent (second pass produces no changes)\n"
    ),
    file = "fix_check.md",
    append = TRUE
  )
} else {
  if (length(parse_error_repos) > 0) {
    cat(
      paste0(
        "**Parse errors introduced by fixes:**\n\n",
        paste0(
          "- [`",
          parse_error_repos,
          "`](https://github.com/",
          parse_error_repos,
          "/tree/",
          all_repos[parse_error_repos],
          ")",
          collapse = "\n"
        ),
        "\n\n"
      ),
      file = "fix_check.md",
      append = TRUE
    )
  }

  if (length(idempotency_repos) > 0) {
    cat(
      paste0(
        "**Non-idempotent fixes (second pass changed files):**\n\n",
        paste0(
          "- [`",
          idempotency_repos,
          "`](https://github.com/",
          idempotency_repos,
          "/tree/",
          all_repos[idempotency_repos],
          ")",
          collapse = "\n"
        ),
        "\n\n"
      ),
      file = "fix_check.md",
      append = TRUE
    )
  }

  n_passed <- length(all_repos) -
    length(unique(c(
      parse_error_repos,
      idempotency_repos
    )))
  if (n_passed > 0) {
    cat(
      paste0(n_passed, " other repo(s) passed all checks.\n"),
      file = "fix_check.md",
      append = TRUE
    )
  }

  quit(status = 1)
}
