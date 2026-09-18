source("docs/make_docs_helpers.R")

### reset docs/rules directory ----
if (dir.exists("docs/rules")) {
  unlink("docs/rules", recursive = TRUE)
}
dir.create("docs/rules")

### get list of lint rules ----

lints_dir <- "crates/jarl-core/src/lints" #
lints <- list.dirs(lints_dir, full.names = FALSE)

# rules are nested 2 levels deep within lints, ignore any subfolders
rules <- lints[lengths(strsplit(lints, split = "/", fixed = TRUE)) == 2]

### Create individual qmd files for rules ---

docs <- logical(length(rules))

for (i in seq_along(rules)) {
  doc_out <- create_rule_md(rules[i], lints_dir)
  docs[i] <- doc_out
}

create_config_md(lints_dir)

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
