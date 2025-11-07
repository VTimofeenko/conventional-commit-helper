Generate [conventional commits][1] subject from a CLI.

# Features

* Commit types (feat/fix/etc.) suggestion
* Commit scope suggestion:
    * From git history
    * From configuration file
    * If the staged files look like they match a scope from history — that scope
      will be suggested (can be cached, see `cache` commands)

* Per-repo configuration of scopes/types
* Composable with other tools ([examples](./docs/SAMPLE.md))

# Limitations/roadmap

- Distribution: only accessible through nix (will not be done unless someone
  actually needs this)
- Scopes: Scopes are checked only for the current branch (most likely will not be done)
- General: bare repositories are not supported (most likely will not be done)

# Usage

<!-- ```$ as shell
nix run . -- --help
``` -->

```shell
Tiny helper for conventional commits (https://www.conventionalcommits.org)

Usage: conventional-commit-helper [OPTIONS] [COMMAND]

Commands:
  cache  Cache operations
  type   Show commit types
  scope  Show commit scopes
  help   Print this message or the help of the given subcommand(s)

Options:
      --repo-path <REPO_PATH>  Path to the non-bare git repository [default: .]
      --config <CONFIG>        Path to a custom config file
  -v, --verbose...             Increase logging verbosity
  -q, --quiet...               Decrease logging verbosity
  -h, --help                   Print help
  -V, --version                Print version
```


# Running

## Nix

Project comes with a [nix flake][2], so it's runnable just as:

```
nix run github:VTimofeenko/conventional-commit-helper
```

The default package can be added to a Nix system configuration.

# Configuration

This program searches per-repo configuration file under
`.dev/conventional-commit-helper.toml`:

```toml
[scopes]
# key/value pairs
foo = "bar"

[types]
# Only these types will be suggested
# key/value pairs
feat = "Some custom description for feat type"
fix = "Some custom description for fix type"
```

[1]: https://www.conventionalcommits.org/en/v1.0.0/
[2]: https://wiki.nixos.org/wiki/Flakes
