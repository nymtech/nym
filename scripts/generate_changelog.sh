#!/bin/bash
# set CHANGELOG_GITHUB_TOKEN in your .bashrc file
# For each version, you can add a release summary with text, images, gif animations, etc, and show new features and notes clearly to the user. This is done using GitHub metadata.

# Example: adding the release summary for v1.0.0:

# 1. Create a new GitHub Issue
# 2. In the Issue's Description field, add your release summary content 
# 3. Set the Issue Label `release-summary` and add it to the GitHub Milestone `v1.0.0`
# 4. Close the Issue and execute `github-changelog-generator`
github_changelog_generator -u nymtech -p nym --exclude-tags 0.1.0 --token "$CHANGELOG_GITHUB_TOKEN"
