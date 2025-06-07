{
  description = "Rust cross compilation shell for Kindle KT2 (armv7 musl)";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable"; # or your preferred nixpkgs

  outputs = { self, nixpkgs }: 
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      # devShells.${system}.default = pkgs.pkgsCross.armv7l-hf-multiplatform.mkShell {
      devShells.${system}.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          cargo-cross
          rustup
          clang
          pkg-config
        ];

        shellHook = ''
          echo "HELLO RUST"
          export RUSTUP_HOME=$PWD/.rustup
          export CARGO_HOME=$PWD/.cargo
          export PATH=$CARGO_HOME/bin:$PATH

          if ! command -v rustup > /dev/null; then
            echo "Rustup not found in PATH!"
            return 1
          fi

          # Ensure rustup default stable is set
          rustup default stable

          # Add target if missing
          if ! rustup target list --installed | grep -q armv7-unknown-linux-musleabihf; then
            echo "Adding armv7-unknown-linux-musleabihf target..."
            rustup target add armv7-unknown-linux-musleabihf
          fi
          rustup component add rust-analyzer

          echo "Ready to cross-compile to armv7-unknown-linux-musleabihf."
          # export RUSTFLAGS="-C target-feature=+crt-static"
          export NOT_KINDLE=1 # not for build but runtime with `cargo run`
          echo 'Run:'
          echo 'RUSTFLAGS="-C target-feature=+crt-static" cross build --target armv7-unknown-linux-musleabihf --release'
        '';
      };
    };
}

