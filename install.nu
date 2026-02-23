#!/usr/bin/env nu

# Install miku-overlay: build release binary, install to ~/.local/bin/,
# and add exec line to sway config for auto-launch on login.

const BINARY_NAME = "miku-overlay"
const INSTALL_DIR = ($env.HOME | path join ".local" "bin")
const SWAY_CONFIG = ($env.HOME | path join ".config" "sway" "config")
const EXEC_LINE = "exec ~/.local/bin/miku-overlay"
const EXEC_COMMENT = "# Miku overlay for Helium browser"

def main [] {
    # Build release binary
    print "Building release binary..."
    cargo build --release
    print $"Build complete."

    # Install binary
    let install_path = ($INSTALL_DIR | path join $BINARY_NAME)
    mkdir $INSTALL_DIR
    cp target/release/($BINARY_NAME) $install_path
    chmod 755 $install_path
    print $"Installed to ($install_path)"

    # Add exec line to sway config if not already present
    if ($SWAY_CONFIG | path exists) {
        let config = (open $SWAY_CONFIG --raw)
        if ($config | str contains $EXEC_LINE) {
            print "Sway config already has miku-overlay exec line."
        } else {
            $"\n($EXEC_COMMENT)\n($EXEC_LINE)\n" | save --append $SWAY_CONFIG
            print $"Added exec line to ($SWAY_CONFIG)"
        }
    } else {
        print $"Warning: Sway config not found at ($SWAY_CONFIG), skipping."
    }

    print "Done. Reload sway or log out/in to start miku-overlay."
}
