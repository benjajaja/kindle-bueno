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
      secretKeys = [ "aemet_key" "aemet_station" "openweatherkey" "tide_station_id" ];
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

          # Use pre-extracted secrets (run 'nix develop' first to extract from sops)
          preBuild = let
            sensitivePath = ./sensitive;
          in ''
            mkdir -p sensitive
            ${pkgs.lib.optionalString (builtins.pathExists sensitivePath) ''
              cp -r ${sensitivePath}/* sensitive/
              chmod -R u+w sensitive
            ''}

            # Verify all required secrets exist
            ${pkgs.lib.concatMapStringsSep "\n" (key: ''
              if [ ! -f "sensitive/${key}" ]; then
                echo "ERROR: sensitive/${key} not found!"
                echo "Run 'nix develop' first to extract secrets from sops"
                exit 1
              fi
            '') secretKeys}
          '';

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

          echo "Nix cross-compilation environment for Kindle"
          echo "Build with: nix build"
          echo "Result will be in: ./result/bin/kindle-bueno"
        '';
      };
    };
}

