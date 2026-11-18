# Releasing

Every release is identified in four places that must agree: the `version` in the root
`Cargo.toml`, a `## <version>` section in `CHANGELOG.md`, the git tag `v<version>`, and the build
metadata inside the binary (`hw-watcher --version` prints the version and commit).

## Steps

1. Update `version` under `[workspace.package]` in `Cargo.toml` and run `cargo build` so
   `Cargo.lock` follows.
2. Add a `## <version>` section to `CHANGELOG.md`. Its text becomes the release notes
   (`scripts/release-notes.sh <version>` previews them).
3. Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings` and
   `cargo test --workspace`.
4. Commit, push and wait for CI to pass.
5. Tag and push: `git tag v<version> && git push origin v<version>`.

The release workflow then:

- fails if the tag does not match `Cargo.toml` or the CHANGELOG section is missing,
- builds on Ubuntu 22.04 for x86_64 and aarch64 with `HW_WATCHER_COMMIT` set to the tagged commit,
- publishes `hw-watcher-<version>-linux-<arch>.tar.gz` and a `.sha256` file, each tarball holding
  the binary, the `hww` alias, docs, scripts, packaging files and `BUILD-INFO.txt`,
- signs a build provenance attestation for each tarball (verify with `gh attestation verify`),
- marks `0.x` releases as pre-releases.

Tag releases with a signed tag: `git tag -s v<version> -m "HW Watcher <version>"`.

Binaries you share must be built with `scripts/build-release.sh`, which strips your home directory
and username from the paths Rust embeds in panic messages (a plain `cargo build --release` does
not). CI uses the same script.

A binary built locally without git history reports `commit unknown`.
