# Development notes

## Make a new release or a new pre-release

1. Bump the version number in all `Cargo.toml` files and in `Cargo.lock` (automatically changed). For pre-releases, use for instance `x.y.z-alpha.1`.
1. Manually trigger the "Release" workflow and give it the same version number. If the version number contains "alpha", `cargo-dist` will automatically create a pre-release instead of a release.
