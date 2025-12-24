# Auto-Generated Documentation

This directory contains automatically generated documentation that is kept up-to-date with the codebase.

## Files

- **`cli-commands.md`** - Complete reference of all `halvor` CLI commands and options
- **`docker-containers.md`** - Available Docker containers and how to use them
- **`helm-charts.md`** - Available Helm charts and installation instructions

## Regenerating Documentation

To regenerate these docs locally:

```bash
make docs
```

Or run the script directly:

```bash
./scripts/generate-docs.sh
```

## Automatic Updates

Documentation is automatically regenerated on every push to the `main` branch via GitHub Actions. If changes are detected, they are automatically committed back to the repository.

## Note

These files are tracked in git to ensure they're always available, even if the generation script fails. However, manual edits to these files will be overwritten on the next generation.

