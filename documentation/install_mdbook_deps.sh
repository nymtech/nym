#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

# simple script to automate cleaning an existing mdbook install then installing it fresh for each deploy.

# pinning minor version allows for updates but no breaking changes
#MINOR_VERSION=0.4
# if a new plugin is added to the books it needs to be added here also
declare -a plugins=("admonish" "linkcheck" "last-changed" "theme" "variables" "cmdrun")

# install mdbook + plugins
install_mdbook_deps() {
	printf "\ninstalling mdbook..."
	# installing mdbook with only specific features for speed
	#  cargo install mdbook --no-default-features --features search --vers "^$MINOR_VERSION"
	# cargo install mdbook --vers "$MINOR_VERSION"
	cargo install mdbook --features search

	printf "\ninstalling plugins..."
	for i in "${plugins[@]}"; do
		cargo install mdbook-$i
	done

	# mdbook-admonish config
	#	if [ $(pwd | awk -F/ '{print $NF}') != "documentation" ]; then
	#		printf "not in documentation/ - changing dir but something isn't right in the workflow file"
	#  		cd documentation/
	#		mdbook-admonish install dev-portal
	#        	mdbook-admonish install docs
	#        	mdbook-admonish install operators
	#	else
	#        mdbook-admonish install dev-portal
	#        mdbook-admonish install docs
	#        mdbook-admonish install operators
	#	fi
}

# uninstall mdbook + plugins
uninstall_mdbook_deps() {
	# mdbook
	printf "\nuninstalling existing mdbook installation...\n"
	cargo uninstall mdbook
	# check it worked
	if [ $? -ne 0 ]; then
		printf "\nsomething went wrong, exiting"
		exit 1
	else
		printf "\nmdbook deleted\n"
	fi

	# plugins
	printf "\nuninstalling existing plugins...\n"
	for i in "${plugins[@]}"; do
		cargo uninstall mdbook-$i
		# check it worked
		if [ $? -ne 0 ]; then
			printf "\nsomething went wrong, exiting"
			exit 1
		else
			printf "\nmdbook-$i deleted\n"
		fi
	done
}

main() {
	if test -f ~/.cargo/bin/mdbook; then
		printf "mdbook already installed (located at: $(which mdbook))"
		uninstall_mdbook_deps
		install_mdbook_deps
	else
		printf "mdbook not installed"
		install_mdbook_deps
	fi
}

main
