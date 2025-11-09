suppressPackageStartupMessages({
  library(data.table)
  library(jsonlite)
  library(tinytable)
})

all_files <- list.files(
  "results_bench",
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

cat("### Benchmark on real-projects\n\n", file = "benchmark.md")

list_results <- list()

for (i in seq_along(all_repos)) {
  repos <- names(all_repos)[i]
  repos_sha <- all_repos[[i]]

  message("Processing results of ", repos)
  main_results_json <- jsonlite::read_json(paste0(
    "results_bench/",
    gsub("/", "_", repos),
    "_main.json"
  ))[["results"]][[1]][["times"]]
  pr_results_json <- jsonlite::read_json(paste0(
    "results_bench/",
    gsub("/", "_", repos),
    "_pr.json"
  ))[["results"]][[1]][["times"]]

  main_mean <- mean(unlist(main_results_json))
  pr_mean <- mean(unlist(pr_results_json))

  list_results[[i]] <- data.frame(
    Repository = repos,
    "Avg. duration (main, seconds)" = main_mean,
    "Avg. duration (PR, seconds)" = pr_mean,
    "PR - main" = pr_mean - main_mean,
    "PR - main (%)" = (pr_mean - main_mean) / main_mean * 100,
    "Number of iterations" = length(main_results_json),
    check.names = FALSE
  )
}

all_results <- rbindlist(list_results)

tt(all_results) |>
  theme_markdown(style = "gfm") |>
  save_tt(output = "markdown") |>
  cat(file = "benchmark.md", append = TRUE)
