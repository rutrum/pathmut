{
  description = "pathmut";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    # for nightly rust version
    rust-overlay.url = "github:oxalica/rust-overlay";

    # build rust packages with nix
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, ... }@inputs: let
    name = "pathmut";
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      overlays = [ (import inputs.rust-overlay) ];
    };
    rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
    craneLib = (inputs.crane.mkLib pkgs).overrideToolchain (p: rust);
  in
  {
    packages.${system}.default = craneLib.buildPackage {
      src = craneLib.cleanCargoSource ./.;
      strictDeps = true;
    };

    devShells.${system}.default = pkgs.mkShell {
      inherit name;
      buildInputs = with pkgs; [
        rust
        just
        watchexec
      ];
    };
  };
}
