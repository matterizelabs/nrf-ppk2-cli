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
          toolchain = fenix.packages.${system}.combine [
            fenix.packages.${system}.stable.toolchain
            fenix.packages.${system}.targets.aarch64-unknown-linux-gnu.stable.toolchain
          ];
          aarch64CrossGcc = pkgs.pkgsCross.aarch64-multiplatform.buildPackages.gcc;
          aarch64Udev = pkgs.pkgsCross.aarch64-multiplatform.udev;
          linuxLibs = with pkgs; lib.optionals stdenv.isLinux [ udev aarch64CrossGcc ];
          darwinLibs = with pkgs; lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.IOKit
            darwin.apple_sdk.frameworks.Foundation
          ];
        in {
          default = pkgs.mkShell {
            nativeBuildInputs = [ toolchain pkgs.pkg-config ];
            buildInputs = linuxLibs ++ darwinLibs;
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath linuxLibs;
            shellHook = ''
              echo "PPK2 dev shell | Rust $(rustc --version)"
              export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-L ${aarch64Udev}/lib"
            '';
          };
        }
      );
    };
}
