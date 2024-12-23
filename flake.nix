{
  description = "pathmut";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs, ... }@inputs: let
    name = "pathmut";
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
  in
  {
    devShells.${system}.default = pkgs.mkShell {
      inherit name;
      buildInputs = with pkgs; [
        rustup
        just
        watchexec
      ];
    };
  };
}
