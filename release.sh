#!/usr/bin/env bash

set -e

if [ -z "$1" ]; then
	echo "Please provide a tag."
	echo "Usage: ./release.sh v[X.Y.Z]"
	exit
fi

echo "Preparing $1..."
# update the version
msg="# bumped by release.sh"
sed -E -i "s/^version = .* $msg$/version = \"${1#v}\" $msg/" Cargo.toml
cargo build
# update the changelog
git cliff --tag "$1" >CHANGELOG.md
git add -A
git commit -m "chore(release): prepare for $1"
git show
# generate a changelog for the tag message
changelog=$(git cliff --tag "$1" --unreleased --strip all)
# create a signed tag
# https://keyserver.ubuntu.com/pks/lookup?search=0x1B250A9F78535D1A&op=vindex
git -c user.name="runst" \
	-c user.email="runstify@proton.me" \
	-c user.signingkey="AEF8C7261F4CEB41A448CBC41B250A9F78535D1A" \
	tag -f -s -a "$1" -m "Release $1" -m "$changelog"
git tag -v "$1"
echo "Done!"
echo "Now push the commit (git push) and the tag (git push --tags)."
