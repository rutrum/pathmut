{
  description = "pathmut";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs, ... }@inputs: let
    name = "pathmut";
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};

    overrides = pkgs.lib.importTOML ./rust-toolchain.toml;
  in
  {
    devShells.${system}.default = pkgs.mkShell {
      inherit name;
      buildInputs = with pkgs; [
        rustup
        #rustPlatform
        just
        watchexec
      ];

      # shouldn't be required?
      #RUSTC_VERSION = overrides.toolchain.channel;
    };
  };
}
