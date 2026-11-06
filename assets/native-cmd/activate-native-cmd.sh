#!/usr/bin/env bash

set -e

# Resolve this script's absolute directory from $0. The naive
# "$0 -> %/*" derivation used to break in real layouts: a bare-name launch
# ("activate-native-cmd.sh" from usr/bin, as the portable first-run
# activation does) yields script_dir=".", whose basename is not "bin", so
# the root was mis-set to the usr/bin directory itself and `wpm links
# rebuild --root .` created every command link under usr/bin/usr/bin.
# Canonicalizing to an absolute path first fixes that, and keeps paths
# with spaces, non-ASCII characters, and UNC prefixes working because
# every expansion below stays quoted.
script_path="$0"
# Normalize separators so parsing below works for native Windows paths.
script_path=$(printf '%s' "$script_path" | tr '\\' '/')
case "$script_path" in
  /*|[A-Za-z]:/*)
    # Already absolute (POSIX absolute, UNC, or drive-letter path).
    ;;
  */*)
    # Relative with a separator: anchor it at the current directory.
    script_path="$PWD/$script_path"
    ;;
  *)
    # Bare name: resolve through PATH; fall back to the current directory.
    resolved=""
    old_ifs="$IFS"
    IFS=":"
    for dir in $PATH; do
      IFS="$old_ifs"
      [ -n "$dir" ] || dir="."
      if [ -f "$dir/$script_path" ]; then
        resolved="$dir/$script_path"
        break
      fi
    done
    IFS="$old_ifs"
    if [ -n "$resolved" ]; then
      script_path="$resolved"
    else
      script_path="$PWD/$script_path"
    fi
    ;;
esac
script_dir="${script_path%/*}"

if [ -f "$script_dir/native-cmd.exe" ] && [ "$(basename "$script_dir")" = "bin" ]; then
  native_cmd_bin_dir="$script_dir"
  native_cmd_root="$script_dir/../.."
elif [ -f "$script_dir/native-cmd.exe" ]; then
  native_cmd_bin_dir="$script_dir"
  native_cmd_root="$script_dir"
elif [ -f "./cmd/usr/bin/native-cmd.exe" ]; then
  native_cmd_bin_dir="./cmd/usr/bin"
  native_cmd_root="./cmd"
elif [ -f "./cmd/native-cmd.exe" ]; then
  native_cmd_bin_dir="./cmd"
  native_cmd_root="./cmd"
else
  echo "activate-native-cmd: native-cmd.exe not found"
  echo "Run this script from the Native Cmd usr/bin directory or release root."
  exit 1
fi

native_cmd_exe="$native_cmd_bin_dir/native-cmd.exe"
mode="create"
link_flag=""

case "$1" in
  --remove)
    mode="remove"
    ;;
  --symbolic)
    link_flag="-s"
    ;;
  --help|-h)
    echo "Usage: activate-native-cmd.sh [--remove] [--symbolic]"
    echo
    echo "Creates command links in usr/bin so ls/cat/grep/etc"
    echo "resolve through PATH when native-shell starts."
    echo "Modern Native Cmd builds use WPM for link discovery; older builds"
    echo "fall back to this script's bundled command list."
    echo
    echo "Options:"
    echo "  --remove    Remove generated command links"
    echo "  --symbolic  Create symbolic links instead of hard links"
    exit 0
    ;;
  "")
    ;;
  *)
    echo "activate-native-cmd: unknown option: $1"
    echo "Try: activate-native-cmd.sh --help"
    exit 1
    ;;
esac

if [ "$link_flag" = "" ] && "$native_cmd_exe" wpm version >/dev/null 2>&1; then
  if [ "$mode" = "remove" ]; then
    "$native_cmd_exe" wpm links remove --root "$native_cmd_root"
  else
    "$native_cmd_exe" wpm links rebuild --root "$native_cmd_root" --force
  fi
  exit $?
fi

commands="
arch b2sum base32 base64 basename basenc cal cat
chcon chgrp chmod chown chroot cksum clear cmp col column comm cp cpio csplit
cut cygpath d2u date dd df diff diff3 dirname
dir dircolors dos2unix du echo env expand expr factor false file
find fmt fold free getconf grep groups head hexdump
hmac256 hostid hostname id infocmp install join jq
kill less link ln locale logname ls lsof man md5sum
mkdir mkfifo mknod mktemp more mpicalc mv nice nl nohup
nproc numfmt od paste patch pathchk pinky pr
printenv printf ps ptx pwd readlink realpath reset
rev rm rmdir runcon sdiff sed seq sha1sum sha224sum
sha256sum sha384sum sha512sum shred shuf sleep sort
split stat stdbuf strings stty sum sync tac tail tee test
[ tic timeout toe top touch tput tr tree true
truncate tsort tty tzset u2d uname unexpand uniq
unix2dos unlink uptime users vdir watch wc which who
whoami xargs xxd yes
"

created=0
removed=0
failed=0

if [ "$mode" != "remove" ]; then
  if [ -f "$native_cmd_bin_dir/ls.exe" ] && [ -f "$native_cmd_bin_dir/cat.exe" ] && [ -f "$native_cmd_bin_dir/grep.exe" ] && [ -f "$native_cmd_bin_dir/ln.exe" ]; then
    echo "Native Cmd command links are already active."
    exit 0
  fi
fi

if [ "$mode" = "remove" ]; then
  echo "Removing Native Cmd command links from $native_cmd_bin_dir"
  for cmd in $commands; do
    target="$native_cmd_bin_dir/$cmd.exe"
    if [ -f "$target" ]; then
      "$native_cmd_exe" rm -f "$target" || failed=$((failed + 1))
      removed=$((removed + 1))
    fi
  done
  echo "Removed: $removed"
else
  echo "Creating Native Cmd command links in $native_cmd_bin_dir"
  for cmd in $commands; do
    target="$native_cmd_bin_dir/$cmd.exe"
    if [ "$link_flag" = "-s" ]; then
      "$native_cmd_exe" ln -s -f "$native_cmd_exe" "$target" || failed=$((failed + 1))
    else
      "$native_cmd_exe" ln -f "$native_cmd_exe" "$target" || failed=$((failed + 1))
    fi
    created=$((created + 1))
  done
  echo "Created: $created"
fi

if [ "$failed" -ne 0 ]; then
  echo "Failed: $failed"
  exit 1
fi

echo "Done."
