library(yaml)

if (dir.exists("docs/rules")) {
  unlink("docs/rules", recursive = TRUE)
}
dir.create("docs/rules")

rule_dirs <- list.files("crates/jarl-core/src/lints", full.names = TRUE)
rule_dirs <- rule_dirs[!grepl("mod.rs", rule_dirs)]
rule_names <- basename(rule_dirs)

### Create individual qmd files for rules

rule_files <- paste0(rule_dirs, "/", rule_names, ".rs")

docs <- lapply(rule_files, \(x) {
  content <- readLines(x)
  if (!any(grepl("## What it does", content, fixed = TRUE))) {
    return()
  }

  start <- grep("## What it does", content, fixed = TRUE)
  end <- grep("(impl Violation for)|(pub fn)", content) - 1
  end <- end[1] # could be several "pub fn"

  doc <- content[start:end]
  doc <- gsub("^///(| )", "", doc)
  # doc <- gsub("^```r", "```\\{r\\}", doc)

  doc
})

empty_docs <- lengths(docs) == 0
docs <- docs[!empty_docs]
rule_names <- rule_names[!empty_docs]
names(docs) <- rule_names

for (i in seq_along(docs)) {
  to_write <- c(paste0("# ", rule_names[i]), docs[[i]])
  writeLines(to_write, paste0("docs/rules/", rule_names[i], ".md"))
}

### Automatically add new rules in _quarto.yml

# Not the same as `rule_names` since we discarded those that don't have any
# docs yet
doc_names <- sort(rule_names)

quarto_yml <- read_yaml("docs/_quarto.yml")
quarto_yml$website$sidebar[[1]]$contents <- list(
  "rules.qmd",
  list(section = "List of rules", contents = paste0("rules/", doc_names, ".md"))
)
quarto_yml$filters <- list("newpagelink.lua")
write_yaml(
  quarto_yml,
  "docs/_quarto.yml",
  handlers = list(
    logical = verbatim_logical
  )
)
