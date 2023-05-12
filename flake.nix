{
  description = "Development environment for nightly cargo";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, nixpkgs }:
    let 
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
    devShells.${system}.default = with pkgs; mkShell {
      buildInputs = [
        cargo
      ];
    };
  };
}
