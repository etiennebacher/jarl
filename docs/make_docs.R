library(fs)

rule_dirs <- list.files("src/lints", full.names = TRUE)
rule_dirs <- rule_dirs[!grepl("mod.rs", rule_dirs)]

rule_names <- basename(rule_dirs)

rule_files <- paste0(rule_dirs, "/", rule_names, ".rs")

docs <- lapply(rule_files, \(x) {
  content <- readLines(x)
  if (!any(grepl("## What it does", content))) {
    return()
  }

  start <- grep("## What it does", content)
  end <- grep("impl Violation for", content) - 1

  doc <- content[start:end]
  doc <- gsub("^///(| )", "", doc)
  doc <- gsub("^```r", "```\\{r\\}", doc)

  doc
})

names(docs) <- rule_names

for (i in seq_along(docs)) {
  if (length(docs[[i]]) == 0) {
    next
  }
  to_write <- c(paste0("# `", rule_names[i], "`"), docs[[i]])
  writeLines(to_write, paste0("docs/rules/", rule_names[i], ".qmd"))
}
