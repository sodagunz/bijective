{
  description = "Surject dev flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs =
    { nixpkgs, ... }:
    {
      devShells.aarch64-darwin.default = nixpkgs.legacyPackages.aarch64-darwin.mkShell {
        buildInputs = [
          nixpkgs.legacyPackages.aarch64-darwin.cargo
          nixpkgs.legacyPackages.aarch64-darwin.rustc
          nixpkgs.legacyPackages.aarch64-darwin.rust-analyzer
        ];
      };

    };
}
