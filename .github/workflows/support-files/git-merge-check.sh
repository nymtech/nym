#!/usr/bin/env bash

#
# Basic usage:
#   ./git-merge-check origin/develop origin/release/v1.1.9
#

set -x

set -o nounset
set -o pipefail

# Start from branch ...
branch1=$1

# ... and try to merge in
branch2=$2

echo "Checking if $branch2 merges without conflicts into $branch1..."

git checkout -q $branch1 || exit 1

# There are two options here as far as I see on what we should check for. Either
#
#  (A) check for CONFLICT in any file except whitelist (such as .lock files)
#  (B) check for "Automatic merge failed"
#
# Both of these options have drawbacks.
#
# The first (A) has the problem in that maybe we don't want to fail if the
# changes can be automatically merged (duh), but at least we are not pestered
# about constant lock file changes.
# The second (B) has the problem that it's difficult to filter out automatic
# merge fails for files we don't care about (.lock files).
#
# The ideal solution would be to check for automatic merge fails for files
# except those on a whitelist (e.g. lock files).

# For now I choose to use (B) here, because I hope it might be less noisy

# Alternative A
#output=$(git merge --no-commit --no-ff $branch2 | grep -v .lock)
#merge_failed=$(echo $output | grep -v "CONFLICT")
#return_code=$?

# Alternative B
output=$(git merge --no-commit --no-ff $branch2)
merge_failed=$(echo $output | grep -v "Automatic merge failed")
return_code=$?

# Restore

git merge --abort
git checkout -q -

if [ $return_code -eq 0 ]; then
    echo "Merge check success"
else
    echo "Merge check failed"
fi

exit $return_code
