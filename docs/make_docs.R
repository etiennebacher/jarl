source("docs/make_docs_helpers.R")

### reset docs/rules directory ----
if (dir.exists("docs/rules")) {
  unlink("docs/rules", recursive = TRUE)
}
dir.create("docs/rules")

### get list of lint rules ----

lints_dir <- "crates/jarl-core/src/lints"
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

### Create the home page from the README

create_index_md()

### Automatically add new rules in zensical.toml

rule_docs <- list.files(
  "docs/rules",
  pattern = "\\.md$"
)

update_rules_nav(paste0("rules/", sort(rule_docs)))
