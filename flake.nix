{
  description = "Rust cross compilation for Kindle KT2 (armv7 musl)";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};

      # Cross-compilation package set for armv7 with musl
      pkgsCross = pkgs.pkgsCross.armv7l-hf-multiplatform.pkgsStatic;
    in
    {
      packages.${system} = {
        default = pkgsCross.rustPlatform.buildRustPackage {
          pname = "kindle-bueno";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          # Copy sensitive/ directory into build since it's gitignored
          preBuild = ''
            cp -r ${./sensitive} sensitive
            chmod -R u+w sensitive
          '';

          # Static linking flags
          RUSTFLAGS = "-C target-feature=+crt-static";

          # Build for armv7 musl
          CARGO_BUILD_TARGET = "armv7-unknown-linux-musleabihf";

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgsCross; [
            # Add any system dependencies here if needed
          ];

          # Don't run tests during cross-compilation
          doCheck = false;
        };
      };

      devShells.${system}.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          cargo
          rustc
          pkg-config
        ];

        shellHook = ''
          export NOT_KINDLE=1
          echo "Nix cross-compilation environment for Kindle"
          echo "Build with: nix build"
          echo "Result will be in: ./result/bin/kindle-bueno"
        '';
      };
    };
}

