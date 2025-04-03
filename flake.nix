# This is based on viper's article https://ayats.org/blog/nix-rustup

{
  description = "Minimal starting project for nix-based maturin package development";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    utils.url = "github:numtide/flake-utils";
    devshell.url = "github:numtide/devshell";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, utils, devshell, rust-overlay, ... }@inputs: {
    overlays.default = final: prev: {
      pythonPackagesExtensions = prev.pythonPackagesExtensions ++ [
        (py-final: py-prev: {
          # maturin-basics = py-final.callPackage ./nix/pkgs/self {};
        })
      ];
    };
  } // utils.lib.eachDefaultSystem (system: {
    # The main development environment
    devShells.default =
      let pkgs = import nixpkgs {
            inherit system;

            overlays = [
              devshell.overlays.default
              rust-overlay.overlays.default
            ];
          };

          pythonEnv = pkgs.python3.withPackages (ps: with ps; [
            numpy
          ]);

          python = pkgs.python3;
          rust = pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" ];
            targets = [ "x86_64-unknown-linux-gnu" ];
          };

      in pkgs.devshell.mkShell {
        name = "maturin-basics";

        commands = with pkgs; [
          { name = "maturin"; package = maturin; }
          { name = "python"; package = pythonEnv; }
        ];

        packages = [
          (pkgs.rust-bin.beta.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        })
        ];

        env = [
          {
            name = "RUST_BACKTRACE";
            value = "full";
          }
        ];
      };

    # The development environment for testing the resulting python package.
    devShells.test = let pkgs = import nixpkgs {
      inherit system;
      overlays = [ devshell.overlays.default self.overlays.default ];
    }; in pkgs.devshell.mkShell {
      name = "maturin-basics-test";

      commands = with pkgs; [
        {
          name = "python"; package = pkgs.python3.withPackages (ps: with ps; [
            # maturin-basics
          ]);
        }
      ];
    };
  });
}
