# Initialize the powercycle environment for fish.
# Usage:
#
#   source powercycle-init.fish
#

set -gx POWERCYCLE_ROOT (realpath (status dirname))
set -gx POWERCYCLE_CONFIG "$POWERCYCLE_ROOT/powercycle.toml"
set -gx POWERCYCLE_LOCAL_CONFIG "$POWERCYCLE_ROOT/powercycle.local.toml"

if not test -e "$POWERCYCLE_LOCAL_CONFIG"
    cp \
        "$POWERCYCLE_ROOT/powercycle.local.toml.example" \
        "$POWERCYCLE_LOCAL_CONFIG"
end

mkdir -p "$POWERCYCLE_ROOT/bin"
mkdir -p "$POWERCYCLE_ROOT/data/runs"
mkdir -p "$POWERCYCLE_ROOT/data/dumps"
mkdir -p "$POWERCYCLE_ROOT/data/certificates"

# bin/ is for hand-placed symlinks; target/release is where cargo puts the
# tools that README.md tells you to build.
fish_add_path --prepend "$POWERCYCLE_ROOT/target/release"
fish_add_path --prepend "$POWERCYCLE_ROOT/bin"

echo "powercycle environment"
echo "  root   : $POWERCYCLE_ROOT"
echo "  config : $POWERCYCLE_CONFIG"
echo "  local  : $POWERCYCLE_LOCAL_CONFIG"
echo "  bin    : $POWERCYCLE_ROOT/bin"
echo "  tools  : $POWERCYCLE_ROOT/target/release"
