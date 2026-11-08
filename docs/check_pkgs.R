reqd_pkgs <- c("reactable", "rmarkdown", "yaml12")
reqd_installed <- reqd_pkgs %in% installed.packages()[, "Package"]

if (!all(reqd_installed)) {
  stop(
    "Packages necessary for building documentation are missing: ",
    paste0(reqd_pkgs[which(!reqd_installed)], collapse = ", ")
  )
}
