#!/bin/bash
# set CHANGELOG_GITHUB_TOKEN in your .bashrc file
github_changelog_generator -u nymtech -p nym --exclude-tags 0.1.0 --token "$CHANGELOG_GITHUB_TOKEN"
