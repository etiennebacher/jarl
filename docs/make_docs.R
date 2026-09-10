### reset rules directory
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

rules <- rules[!grepl("(mod|options).rs", rules)]

### Create individual qmd files for rules

create_doc <- function(rule) {
  content <- readLines(rule)
  rule_name <- gsub("\\.rs$", "", basename(rule))

  if (!any(grepl("## What it does", content, fixed = TRUE))) {
    return(FALSE)
  }

  added_in_version <- grep("/// Version added:", content, value = TRUE)
  added_in_version <- gsub(
    "/// Version added: (\\d\\.\\d\\.\\d)",
    "\\1",
    added_in_version
  )

  if (
    length(added_in_version) != 1 ||
      !grepl("^\\d+\\.\\d+\\.\\d+$", added_in_version)
  ) {
    stop(
      paste0(
        "Couldn't find the 'Version added' line for rule '",
        rule,
        "'."
      )
    )
  }

  start <- grep("## What it does", content, fixed = TRUE)
  end <- grep("^(impl Violation for|fn |pub fn|// )", content) - 1
  end <- end[end > start]
  end <- end[1] # could be several "pub fn"

  doc <- content[start:end]
  doc <- gsub("^///(| )", "", doc)

  doc <- c(
    paste0("# ", rule_name),
    paste0(
      '::: {.callout-note title="Added in ',
      added_in_version,
      '" .low-opacity}\n',
      ":::\n"
    ),
    doc
  )

  writeLines(doc, paste0("docs/rules/", rule_name, ".md"))

  return(TRUE)
}

docs <- logical(length(rules))

for (i in seq_along(rules)) {
  doc_out <- create_doc(rules[i])
  docs[i] <- doc_out
}

### Automatically add new rules in _quarto.yml

rule_docs <- list.files(
  "docs/rules",
  pattern = "\\.md$"
)

quarto_yml <- yaml12::read_yaml("docs/_quarto.yml")

quarto_yml$website$sidebar <- list(list(
  title = "Rules",
  style = "floating",
  contents = list(
    "rules.qmd",
    list(
      section = "List of rules",
      contents = paste0("rules/", sort(rule_docs))
    )
  )
))

# use format_yaml until https://github.com/posit.dev/r-yaml12/issues#40 fixed
writeLines(
  yaml12::format_yaml(quarto_yml),
  "docs/_quarto.yml"
)
