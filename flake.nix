{
  description = "A small CLI to help writing conventional commits";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      perSystem =
        { pkgs, ... }:
        let
          craneLib = inputs.crane.mkLib pkgs;
        in
        {
          packages.default = craneLib.buildPackage { src = craneLib.cleanCargoSource ./.; };
          devShells.default = craneLib.devShell { };
        };
    };
}