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

- Scopes: no way to disable scope search in history (to be done)
- Scopes: no way to hide scopes from suggestions (to be done)
- Configuration: no global config file (to be done)
- Configuration: no support for alternative config location, so the config needs
  to be checked in or added to gitignore/exclude (to be done)
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

Usage: conventional-commit-helper [OPTIONS] [MODE]

Arguments:
  [MODE]  [possible values: type, scope]

Options:
      --json                   Print output in JSON format
      --repo-path <REPO_PATH>  Path to the non-bare git repository [default: .]
      --debug
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
