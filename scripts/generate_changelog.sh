# Copyright 2020 - The Nym Mixnet Authors
# SPDX-License-Identifier: Apache-2.0


#!/bin/bash

# Copyright 2020 Nym
# 
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
# 
#     http://www.apache.org/licenses/LICENSE-2.0
# 
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# set CHANGELOG_GITHUB_TOKEN in your .bashrc file
# For each version, you can add a release summary with text, images, gif animations, etc, and show new features and notes clearly to the user. This is done using GitHub metadata.

# Example: adding the release summary for v1.0.0:

# 1. Create a new GitHub Issue
# 2. In the Issue's Description field, add your release summary content 
# 3. Set the Issue Label `release-summary` and add it to the GitHub Milestone `v1.0.0`
# 4. Close the Issue and execute `github-changelog-generator`
github_changelog_generator -u nymtech -p nym --exclude-tags 0.1.0,nym-wallet-v1.0.0-windows --token "$CHANGELOG_GITHUB_TOKEN"
