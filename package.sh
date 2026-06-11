#!/bin/bash
# Package encoding-vfs for distribution (Linux version)
# Creates platform-specific archives containing encoding-vfs and git-wrapper

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Get version from Cargo.toml
VERSION=$(grep -m1 'version' encoding-vfs-cli/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')

echo "Packaging encoding-vfs v${VERSION}"
echo ""

# Build
echo "Building..."
cargo build --release

# Create dist directory
DIST_DIR="dist"
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# Package name
ARCHIVE_NAME="encoding-vfs-v${VERSION}-linux-x86_64"
PKG_DIR="$DIST_DIR/$ARCHIVE_NAME"
mkdir -p "$PKG_DIR"

# Copy executables
cp target/release/encoding-vfs "$PKG_DIR/"
cp target/release/git "$PKG_DIR/"

# Copy install script (create a simple one for Linux)
cat > "$PKG_DIR/install-git-wrapper.sh" << 'EOF'
#!/bin/bash
# Install git wrapper for encoding-vfs

INSTALL_DIR="$HOME/.local/bin"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

mkdir -p "$INSTALL_DIR"

# Copy git wrapper
cp "$SCRIPT_DIR/git" "$INSTALL_DIR/git-encoding-vfs"
chmod +x "$INSTALL_DIR/git-encoding-vfs"

# Create alias suggestion
echo ""
echo "Git wrapper installed to: $INSTALL_DIR/git-encoding-vfs"
echo ""
echo "Option 1: Create an alias (add to ~/.bashrc):"
echo "  alias git-vfs='\"$INSTALL_DIR/git-encoding-vfs\"'"
echo ""
echo "Option 2: Add to PATH (if you want to override system git):"
echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
echo ""
EOF
chmod +x "$PKG_DIR/install-git-wrapper.sh"

# Create README
cat > "$PKG_DIR/README.md" << EOF
# Encoding VFS v${VERSION} (Linux)

## Contents

- \`encoding-vfs\` - Main VFS program
- \`git\` - Transparent git wrapper
- \`install-git-wrapper.sh\` - Installation script

## Quick Start

### 1. Mount a project

\`\`\`bash
./encoding-vfs -b /path/to/your/project -m /mnt/vfs
\`\`\`

### 2. Install git wrapper

\`\`\`bash
./install-git-wrapper.sh
\`\`\`

### 3. Use git transparently

\`\`\`bash
cd /mnt/vfs
git status  # Automatically maps to source directory
git log
git diff
\`\`\`

## How it works

The git wrapper reads \`~/.encoding-vfs/mounts.json\` to find active mounts
and automatically redirects git commands to the source directory.

## Requirements

- FUSE3: \`sudo apt-get install fuse3\` (Ubuntu/Debian)
- Or: \`sudo dnf install fuse3\` (Fedora)
EOF

# Create tar.gz archive
cd "$DIST_DIR"
tar -czf "${ARCHIVE_NAME}.tar.gz" "$ARCHIVE_NAME"
cd ..

# Clean up temp directory
rm -rf "$PKG_DIR"

echo ""
echo "Done! Archive created:"
echo "  $DIST_DIR/${ARCHIVE_NAME}.tar.gz"
ls -lh "$DIST_DIR/"
