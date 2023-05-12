{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    let
      overlays = [ (import rust-overlay) ];
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system overlays; };
    in {
      devShell.${system} = pkgs.mkShell {
        name = "dev";
        buildInputs = [
          pkgs.hello
          pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default)
        ];
      };

      packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
        name = "pathmut";
        src = ./.;
        cargoLock = {
          lockFile = ./Cargo.lock;
        };
      };
    };
}
