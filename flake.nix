{
  description = "Nightly Rust environment for building std";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Define the exact toolchain we want: Nightly + rust-src
        rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
          extensions = ["rust-src" "rust-analyzer"];
        };
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.openssl
            pkgs.pkg-config
            # Add any other system dependencies you might need here (e.g., pkg-config, openssl)
          ];

          shellHook = ''
            echo "🦀 Welcome to your Nightly Rust Nix shell!"
            rustc --version
          '';
        };
      }
    );
}
