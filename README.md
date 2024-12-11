Generate [conventional commits][1] message from a CLI

# Features

* Commit types (feat/fix/etc.) suggestion
* Commit scope suggestion:
    * From git history
    * From config
    * Compare staged files to ones that changed in the past in the same scope
* Per-repo configuration of scopes/types
* Composable with other tools (TODO: lazygit example)

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

## Cargo

TODO

# Configuration

TODO

[1]: https://www.conventionalcommits.org/en/v1.0.0/
[2]: https://wiki.nixos.org/wiki/Flakes