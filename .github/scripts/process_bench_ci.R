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

# Name of the branch the PR is compared against, used to label the results.
base_ref <- Sys.getenv("BASE_REF", "main")

repos_raw <- Sys.getenv("TEST_REPOS")
repo_lines <- strsplit(repos_raw, "\n")[[1]]
repo_lines <- repo_lines[repo_lines != ""]
repo_parts <- strsplit(repo_lines, "@")
all_repos <- setNames(
  lapply(repo_parts, function(x) trimws(x[2])), # the commit SHAs
  sapply(repo_parts, function(x) trimws(x[1])) # the repo names
)

cat("### Benchmark on real-life projects\n\n", file = "benchmark.md")

list_results <- list()

for (i in seq_along(all_repos)) {
  repos <- names(all_repos)[i]
  repos_sha <- all_repos[[i]]

  message("Processing results of ", repos)
  base_results_json <- jsonlite::read_json(paste0(
    "results_bench/",
    gsub("/", "_", repos),
    "_base.json"
  ))[["results"]][[1]][["times"]]
  pr_results_json <- jsonlite::read_json(paste0(
    "results_bench/",
    gsub("/", "_", repos),
    "_pr.json"
  ))[["results"]][[1]][["times"]]

  base_mean <- mean(unlist(base_results_json))
  pr_mean <- mean(unlist(pr_results_json))

  list_results[[i]] <- data.frame(
    repository = repos,
    base_mean = base_mean,
    pr_mean = pr_mean,
    diff = pr_mean - base_mean,
    diff_pct = (pr_mean - base_mean) / base_mean * 100,
    n_iterations = length(base_results_json)
  )
}

all_results <- rbindlist(list_results)
setnames(
  all_results,
  c(
    "Repository",
    sprintf("Avg. duration (%s, seconds)", base_ref),
    "Avg. duration (PR, seconds)",
    sprintf("PR - %s", base_ref),
    sprintf("PR - %s (%%)", base_ref),
    "Number of iterations"
  )
)

tt(all_results) |>
  theme_markdown(style = "gfm") |>
  save_tt(output = "markdown") |>
  cat(file = "benchmark.md", append = TRUE)
