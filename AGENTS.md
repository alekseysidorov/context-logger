# AGENTS.md

> **All documentation must be in English. Keep instructions concise.**

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/)
specification.

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Common types

- `feat` — new feature
- `fix` — bug fix
- `docs` — documentation changes
- `refactor` — code refactoring (no functional change)
- `chore` — maintenance, tooling, config
- `test` — adding or updating tests

### Rules

- Type is required; scope and breaking-change marker are optional.
- Use a single blank line between the description and the body.
- Describe changes in imperative mood: "fix parsing issue" not "fixed parsing
  issue".
- Commit message title should be <= 72 characters;
