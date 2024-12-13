Sample integrations of the project's program with other tools.

# Lazygit

[Lazygit][1] is a great TUI that makes everyday git tasks much easier. There is
an example of [conventional commit creation in the documentation][2], but it
requires specifying the settings globally and cannot be overriden on a
per-project basis.

Here's an example of using `conventional-commit-helper` with `lazygit`:

```yaml
- command: 'git commit --message ''{{.Form.Type}}{{ if .Form.Scope }}({{ .Form.Scope
    }}){{ end }}: {{.Form.Message}}'''
  context: global
  description: Create new conventional commit
  key: <c-v>
  loadingText: Creating conventional commit...
  prompts:
  - command: conventional-commit-helper type
    filter: ((?P<c_type>[a-z]*):.*)
    key: Type
    labelFormat: '{{ .group_1 }}'
    title: Type of change
    type: menuFromCommand
    valueFormat: '{{ .c_type }}'
  - initialValue: ''
    key: Scope
    suggestions:
      # NOTE: needs jq since looks like lazygit cannot post-process results of
      # suggestions like it does for 'menuFromCommand'
      command: conventional-commit-helper
        scope --json | jq
        -r '.[] | .name '
    title: Scope
    type: input
  # breaking is usually a "no", so it's dropped for brevity
  - initialValue: ''
    key: Message
    title: message
    type: input
  - body: Are you sure you want to commit?
    key: Confirm
    title: Commit
    type: confirm
```

[1]: https://github.com/jesseduffield/lazygit
[2]: https://github.com/jesseduffield/lazygit/wiki/Custom-Commands-Compendium#conventional-commit
