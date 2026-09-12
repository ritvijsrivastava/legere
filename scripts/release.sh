#!/usr/bin/env bash
set -euo pipefail
type="${1:?Usage: npm run release -- <major|minor|patch>}"
npm version "$type" --no-git-tag-version
v=$(node -p "require('./package.json').version")
git add -A
git commit -m "chore: bump version to v$v"
echo "Bumped to v$v. Run 'git ptag' to tag and push."
