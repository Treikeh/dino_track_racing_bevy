# A shell that allows you to develop with bevy
# This shell assumes that you allready have rustup installed on your system
# Remember to rename this file to "shell.nix"

# Rust analyzer will have a proc-macro error when this shell is used with vscodium.
# The only way I have found to fix it is to open vscodium from the terminal after entering the shell (commands: nix-shell -> codium .)

{ pkgs ? import <nixpkgs> { } }:

with pkgs;

mkShell rec {
  buildInputs = [
    gcc # Not actually necessary since it's automatically added when using the nix-shell command
    rust-analyzer
    pkg-config
    udev

    # For faster compiles, but i don't know if it works
    clang
    mold
    llvmPackages.bintools

    # For hotpatching
    #dioxus-cli  # NixPkgs only has version 0.6.3. Version 0.7.0 and up is needed for hotpatching
    #libressl_4_0 # Is needed to compile dioxus-cli if it is installed with "cargo install dioxus-cli@0.7.0-rc.0 --locked"

    alsa-lib-with-plugins
    vulkan-loader
    #xorg.libX11
    #xorg.libXcursor
    #xorg.libXi
    #xorg.libXrandr # To use the x11 feature
    wayland # To use the wayland feature
    libxkbcommon 
  ];
  LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
  shellHook = ''
    codium .
  '';
}