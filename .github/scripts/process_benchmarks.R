#!/usr/bin/env Rscript

# Process benchmark results from divan
# This script compares benchmark results between main and PR branches

library(jsonlite)

# Get list of test repositories from environment
test_repos <- Sys.getenv("TEST_REPOS")
repos <- strsplit(test_repos, "\n")[[1]]
repos <- repos[repos != ""]

# Initialize results data frame
results <- data.frame(
  repository = character(),
  main_mean_ns = numeric(),
  pr_mean_ns = numeric(),
  main_mean_ms = numeric(),
  pr_mean_ms = numeric(),
  change_pct = numeric(),
  status = character(),
  stringsAsFactors = FALSE
)

# Process each repository
for (repo in repos) {
  repo_parts <- strsplit(repo, "@")[[1]]
  repo_name <- repo_parts[1]
  repo_dir <- gsub("/", "_", repo_name)

  main_file <- file.path("benchmark-results", paste0(repo_dir, "_main.json"))
  pr_file <- file.path("benchmark-results-pr", paste0(repo_dir, "_pr.json"))

  cat("Processing:", repo_name, "\n")

  main_time_ns <- NA
  pr_time_ns <- NA

  # Read main benchmark
  if (file.exists(main_file)) {
    tryCatch({
      # Divan outputs line-delimited JSON with benchmark info
      main_lines <- readLines(main_file, warn = FALSE)

      # Try to find the benchmark results
      # Divan typically outputs timing in nanoseconds
      for (line in main_lines) {
        if (nchar(line) > 0 && startsWith(trimws(line), "{")) {
          json_data <- fromJSON(line, simplifyVector = FALSE)

          # Look for timing information in various possible fields
          if (!is.null(json_data$mean)) {
            main_time_ns <- as.numeric(json_data$mean)
            break
          } else if (!is.null(json_data$time)) {
            main_time_ns <- as.numeric(json_data$time)
            break
          } else if (!is.null(json_data$duration)) {
            main_time_ns <- as.numeric(json_data$duration)
            break
          }
        }
      }
    }, error = function(e) {
      cat("  Error reading main benchmark:", conditionMessage(e), "\n")
    })
  }

  # Read PR benchmark
  if (file.exists(pr_file)) {
    tryCatch({
      pr_lines <- readLines(pr_file, warn = FALSE)

      for (line in pr_lines) {
        if (nchar(line) > 0 && startsWith(trimws(line), "{")) {
          json_data <- fromJSON(line, simplifyVector = FALSE)

          if (!is.null(json_data$mean)) {
            pr_time_ns <- as.numeric(json_data$mean)
            break
          } else if (!is.null(json_data$time)) {
            pr_time_ns <- as.numeric(json_data$time)
            break
          } else if (!is.null(json_data$duration)) {
            pr_time_ns <- as.numeric(json_data$duration)
            break
          }
        }
      }
    }, error = function(e) {
      cat("  Error reading PR benchmark:", conditionMessage(e), "\n")
    })
  }

  # Calculate change and status
  if (!is.na(main_time_ns) && !is.na(pr_time_ns) && main_time_ns > 0) {
    change_pct <- ((pr_time_ns - main_time_ns) / main_time_ns) * 100

    if (change_pct < -5) {
      status <- "🚀 Faster"
    } else if (change_pct > 5) {
      status <- "⚠️ Slower"
    } else {
      status <- "✅ Similar"
    }

    # Convert to milliseconds for readability
    main_ms <- main_time_ns / 1e6
    pr_ms <- pr_time_ns / 1e6

    results <- rbind(results, data.frame(
      repository = repo_name,
      main_mean_ns = main_time_ns,
      pr_mean_ns = pr_time_ns,
      main_mean_ms = main_ms,
      pr_mean_ms = pr_ms,
      change_pct = change_pct,
      status = status,
      stringsAsFactors = FALSE
    ))

    cat("  Main:", sprintf("%.2f ms", main_ms),
        "| PR:", sprintf("%.2f ms", pr_ms),
        "| Change:", sprintf("%+.2f%%", change_pct), "\n")
  } else {
    results <- rbind(results, data.frame(
      repository = repo_name,
      main_mean_ns = NA,
      pr_mean_ns = NA,
      main_mean_ms = NA,
      pr_mean_ms = NA,
      change_pct = NA,
      status = "❌ Missing data",
      stringsAsFactors = FALSE
    ))
    cat("  Could not extract timing data\n")
  }
}

# Generate markdown report
md_lines <- c(
  "# Benchmark Results",
  "",
  "Comparison of linting performance between `main` and this PR.",
  "",
  "| Repository | Main | PR | Change | Status |",
  "|------------|------|-----|--------|--------|"
)

for (i in seq_len(nrow(results))) {
  row <- results[i, ]

  if (!is.na(row$main_mean_ms) && !is.na(row$pr_mean_ms)) {
    main_str <- sprintf("%.2f ms", row$main_mean_ms)
    pr_str <- sprintf("%.2f ms", row$pr_mean_ms)
    change_str <- sprintf("%+.2f%%", row$change_pct)
  } else {
    main_str <- "-"
    pr_str <- "-"
    change_str <- "N/A"
  }

  md_lines <- c(md_lines, sprintf(
    "| `%s` | %s | %s | %s | %s |",
    row$repository,
    main_str,
    pr_str,
    change_str,
    row$status
  ))
}

# Add summary statistics if we have valid results
valid_results <- results[!is.na(results$change_pct), ]
if (nrow(valid_results) > 0) {
  md_lines <- c(md_lines, "", "## Summary", "")

  avg_change <- mean(valid_results$change_pct)
  median_change <- median(valid_results$change_pct)

  faster_count <- sum(valid_results$change_pct < -5)
  slower_count <- sum(valid_results$change_pct > 5)
  similar_count <- sum(abs(valid_results$change_pct) <= 5)

  md_lines <- c(md_lines,
    sprintf("- **Average change**: %+.2f%%", avg_change),
    sprintf("- **Median change**: %+.2f%%", median_change),
    sprintf("- **Faster**: %d repositories", faster_count),
    sprintf("- **Slower**: %d repositories", slower_count),
    sprintf("- **Similar**: %d repositories", similar_count)
  )
}

md_lines <- c(md_lines, "", "---", "*Benchmarks ran on `ubuntu-latest` using divan.*")

# Write markdown file
writeLines(md_lines, "benchmark_comparison.md")

cat("\nMarkdown report written to benchmark_comparison.md\n")

# Also save raw results as CSV for potential further analysis
write.csv(results, "benchmark_results.csv", row.names = FALSE)
cat("Raw results saved to benchmark_results.csv\n")
