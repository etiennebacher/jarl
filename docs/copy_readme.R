if (!nzchar(Sys.getenv("QUARTO_PROJECT_RENDER_ALL"))) {
  quit()
}

fs::file_copy("../CHANGELOG.md", "changelog.md", overwrite = TRUE)
