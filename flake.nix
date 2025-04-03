# This is based on viper's article https://ayats.org/blog/nix-rustup

{
  description =
    "Minimal starting project for nix-based maturin package development";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    utils.url = "github:numtide/flake-utils";
    devshell.url = "github:numtide/devshell";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, utils, devshell, rust-overlay, ... }@inputs:
    {
      overlays.default = nixpkgs.lib.composeManyExtensions [
        (final: prev: {
          pythonPackagesExtensions = prev.pythonPackagesExtensions ++ [
            (python-final: python-prev: {
              dbase = python-final.callPackage ./nix/pkgs/self { };
            })
          ];
        })
      ];
    } // utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            self.overlays.default
            rust-overlay.overlays.default
            devshell.overlays.default
          ];
        };
      in {
        devShells.default = pkgs.devshell.mkShell {
          name = "maturin-basics";

          commands = with pkgs; [
            {
              name = "maturin";
              package = maturin;
            }
            {
              name = "python";
              package =
                pkgs.python3.withPackages (ps: with ps; [ numpy dbfread dbf dbase]);
            }
          ];

          packages = [
            (pkgs.rust-bin.beta.latest.default.override {
              extensions = [ "rust-src" "rust-analyzer" ];
            })
          ];

          env = [{
            name = "RUST_BACKTRACE";
            value = "full";
          }];
        };

        # The development environment for testing the resulting python package.
        devShells.test = let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ devshell.overlays.default self.overlays.default ];
          };

        in pkgs.devshell.mkShell {
          name = "maturin-basics-test";

          commands = with pkgs; [{
            name = "python";
            package = pkgs.python3.withPackages (ps: with ps; [ ]);
          }];
        };

        packages.default = pkgs.python3Packages.dbase;
      });
}
