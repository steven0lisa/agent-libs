#!/bin/bash

# publish.sh - Automated publishing script for @zhangzichao2008/agent-lib
# Features: Build, version update, publish, git commit and tag

set -e

echo "==> Starting automated publishing process..."

# 1. Build project
echo "==> Building project..."
npm run build
echo "==> Build completed"

# 2. Get current version and bump patch version
echo "==> Updating version number..."

CURRENT_VERSION=$(node -p "require('./package.json').version")
echo "  Current version: $CURRENT_VERSION"

IFS='.' read -ra VP <<< "$CURRENT_VERSION"
MAJOR=${VP[0]}
MINOR=${VP[1]}
PATCH=${VP[2]}
NEW_PATCH=$((PATCH + 1))
NEW_VERSION="$MAJOR.$MINOR.$NEW_PATCH"

echo "  New version: $NEW_VERSION"

# Update version in package.json
jq ".version = \"$NEW_VERSION\"" package.json > package.json.tmp && mv package.json.tmp package.json
echo "==> Version updated to: $NEW_VERSION"

# 3. Publish to npm (prepublishOnly hook runs build automatically)
echo "==> Publishing to npm..."
npm publish
echo "==> npm publish successful"

# 4. Git commit the version bump
echo "==> Committing version bump..."
git add package.json package-lock.json
git commit -m "chore: bump version to $NEW_VERSION" || echo "  (no changes to commit)"

# 5. Create and push git tag
echo "==> Creating and pushing tag..."
TAG_NAME="v$NEW_VERSION"
git tag "$TAG_NAME"
git push origin "$(git branch --show-current)" --tags

echo ""
echo "==> Publishing process completed!"
echo "  Package: @zhangzichao2008/agent-lib"
echo "  Version: $NEW_VERSION"
echo "  Tag:     $TAG_NAME"
echo ""
echo "Install with: npm install @zhangzichao2008/agent-lib@$NEW_VERSION"
