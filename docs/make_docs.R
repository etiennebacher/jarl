library(yaml)

if (dir.exists("docs/rules")) {
  unlink("docs/rules", recursive = TRUE)
}
dir.create("docs/rules")

rules <- list.files(
  "crates/jarl-core/src/lints",
  full.names = TRUE,
  recursive = TRUE,
  pattern = "\\.rs$"
)
rules <- rules[!grepl("mod.rs", rules)]
rule_names <- gsub("\\.rs$", "", basename(rules))

### Create individual qmd files for rules

docs <- lapply(seq_along(rules), \(x) {
  content <- readLines(rules[x])
  if (!any(grepl("## What it does", content, fixed = TRUE))) {
    return()
  }

  added_in_version <- grep("/// Version added:", content, value = TRUE)
  added_in_version <- gsub(
    "/// Version added: (\\d\\.\\d\\.\\d)",
    "\\1",
    added_in_version
  )

  if (
    length(added_in_version) != 1 ||
      !grepl("\\d+\\.\\d+\\.\\d+", added_in_version)
  ) {
    stop(
      paste0(
        "Couldn't find the 'Version added' line for rule '",
        rule_names[x],
        "'."
      ),
      call. = FALSE
    )
  }

  start <- grep("## What it does", content, fixed = TRUE)
  end <- grep("^(impl Violation for|fn |pub fn)", content) - 1
  end <- end[end > start]
  end <- end[1] # could be several "pub fn"

  doc <- content[start:end]
  doc <- gsub("^///(| )", "", doc)

  doc <- c(
    paste0(
      '::: {.callout-note title="Added in [',
      added_in_version,
      '](https://github.com/etiennebacher/jarl/releases/tag/',
      added_in_version,
      ')"}',
      '\n',
      ':::\n'
    ),
    doc
  )

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
