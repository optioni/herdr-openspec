# Baseline

Base SHA this change starts from (`git rev-parse HEAD`, captured before any task in this
change touched the tree):

```
47353bfac8db7e6bb4fc4db97be018d65be9600c
```

Tasks 10.3 and 12.9's `git diff --name-only <base>..HEAD -- src/` use this SHA as `<base>`.
