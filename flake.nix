{
  description = "PPK2 CLI development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, fenix }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in {
      devShells = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          toolchain = fenix.packages.${system}.stable;
          linuxLibs = with pkgs; lib.optionals stdenv.isLinux [ udev ];
          darwinLibs = with pkgs; lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.IOKit
            darwin.apple_sdk.frameworks.Foundation
          ];
        in {
          default = pkgs.mkShell {
            nativeBuildInputs = [ toolchain.toolchain pkgs.pkg-config ];
            buildInputs = linuxLibs ++ darwinLibs;
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath linuxLibs;
            shellHook = ''
              echo "PPK2 dev shell | Rust $(rustc --version)"
            '';
          };
        }
      );
    };
}
