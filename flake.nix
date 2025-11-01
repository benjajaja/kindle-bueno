{
  description = "Rust cross compilation for Kindle KT2 (armv7 musl)";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  inputs.sops-nix.url = "github:Mic92/sops-nix";
  inputs.sops-nix.inputs.nixpkgs.follows = "nixpkgs";


  outputs = { self, nixpkgs, sops-nix }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};

      # Cross-compilation package set for armv7 with musl
      pkgsCross = pkgs.pkgsCross.armv7l-hf-multiplatform.pkgsStatic;

      sopsConfig = import ./sops.nix;
      secretKeys = [ "aemet_key" "aemet_station" "tide_station_id" "aemet_prediction_beach" ];
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

          # Read secrets from env vars (requires --impure)
          # Run from within 'nix develop' shell
          preBuild = ''
            mkdir -p sensitive
            ${pkgs.lib.concatMapStringsSep "\n" (key: ''
              echo -n "$SECRET_${pkgs.lib.toUpper (pkgs.lib.replaceStrings ["-"] ["_"] key)}" > sensitive/${key}
            '') secretKeys}
          '';

          SECRET_AEMET_KEY = builtins.getEnv "SECRET_AEMET_KEY";
          SECRET_AEMET_STATION = builtins.getEnv "SECRET_AEMET_STATION";
          SECRET_AEMET_PREDICTION_BEACH = builtins.getEnv "SECRET_AEMET_PREDICTION_BEACH";
          SECRET_TIDE_STATION_ID = builtins.getEnv "SECRET_TIDE_STATION_ID";

          # Static linking flags
          RUSTFLAGS = "-C target-feature=+crt-static";

          # Build for armv7 musl
          CARGO_BUILD_TARGET = "armv7-unknown-linux-musleabihf";

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgsCross; [
            sops-nix.nixosModules.sops
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
        buildInputs = [
          sops-nix.nixosModules.sops
          pkgs.sops
        ];

        shellHook = ''
          export NOT_KINDLE=1

          # Extract secrets from sops secrets.yaml
          if [ ! -d "sensitive" ] || [ "secrets.yaml" -nt "sensitive/aemet_key" ]; then
            echo "Extracting secrets from secrets.yaml..."
            mkdir -p sensitive
            ${pkgs.lib.concatMapStringsSep "\n" (key: ''
              ${pkgs.sops}/bin/sops -d --extract '["${key}"]' secrets.yaml > sensitive/${key}
            '') secretKeys}
            echo "Secrets extracted to sensitive/ directory"
          fi

          # Export secrets as env vars for nix build --impure
          ${pkgs.lib.concatMapStringsSep "\n" (key: ''
            export SECRET_${pkgs.lib.toUpper (pkgs.lib.replaceStrings ["-"] ["_"] key)}=$(cat sensitive/${key})
          '') secretKeys}

          echo "Nix cross-compilation environment for Kindle"
          echo "Build with: nix build --impure"
          echo "Result will be in: ./result/bin/kindle-bueno"
        '';
      };
    };
}

