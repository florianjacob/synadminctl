{ pkgs ? import <nixpkgs> {}, unstable ? import <nixos-unstable> {} }:
pkgs.mkShell {
  buildInputs = [
    ((pkgs.rustChannelOf { channel = "stable"; }).rust.override {
      extensions = [ "clippy-preview" ];
    })
    pkgs.cargo-outdated
    pkgs.openssl
    pkgs.pkgconfig
    pkgs.gcc

  ];

  # move build files and artifacts out of source directory to XDG_CACHE_HOME
  shellHook = ''
    # path of this shell.nix file, escaped by systemd to have a working directory name identifier
    identifier=$(/run/current-system/sw/bin/systemd-escape -p ${toString ./.})
    # all missing directories in $CARGO_TARGET_DIR path are created automatically by cargo
    export CARGO_TARGET_DIR="''${XDG_CACHE_HOME:-$HOME/.cache}/cargo/targets/$identifier"
  '';
}
