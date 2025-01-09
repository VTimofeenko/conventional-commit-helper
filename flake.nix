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
          pkg = craneLib.buildPackage {
            src = craneLib.cleanCargoSource ./.;
            meta.mainProgram = "conventional-commit-helper";
            # My cache_ops test makes some incorrect assumptions about paths
            # that are not true in context of a nix build.
            # In the spirit of just getting it to work, disable the test.
            # TODO: fix the test
            # I will probably need to:
            # 1. Override the HOME variable from /homeless-shelter for the check phase
            # 2. Change the cache_path variable in the test to work better on MacOS
            cargoTestExtraArgs = pkgs.lib.optionalString pkgs.stdenv.isDarwin "-- --skip cache_ops";
          };
        in
        {
          checks = {
            inherit pkg;
          };
          packages.default = pkg;
          devShells.default = craneLib.devShell {
            packages = [ pkgs.mdsh ];
          };
        };
    };
}
