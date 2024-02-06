{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

  inputs.fenix.url = "github:nix-community/fenix";
  inputs.fenix.inputs.nixpkgs.follows = "nixpkgs";

  outputs = { self, nixpkgs, fenix }:
    let
      # Use nixpkgs lib to extend nix library features
      lib = nixpkgs.lib;

      # List of supported systems and abis
      supportedSystems = [ "aarch64-linux" "x86_64-linux" ];

      supportedAbis = [ "gnu" "musl" ];

      # Replace package set with musl equivalent
      mkCrossPkgs = pkgs: abi: import nixpkgs {
        localSystem = pkgs.stdenv.buildPlatform;

        crossSystem.parsed = pkgs.stdenv.targetPlatform.parsed // {
          abi = lib.systems.parse.abis.${abi};
        };
      };

      # Create rust toolchain for specified target
      mkRustToolchain = tpkgs:
        let
          # Use system level fenix tooling
          fpkgs = fenix.packages.${tpkgs.system};

          # Currently pinned by fenix to recent stable
          channel = "stable";

          # Default components and developer tools
          components = [ "cargo" "rustc" "rust-src" "rust-docs" "rust-analyzer" "rustfmt" "clippy" ];

          # Include toolchain for target and wasm32
          targets = [ tpkgs.stdenv.targetPlatform.config "wasm32-unknown-unknown" ];

          # Helper function to retrieve toolchain for each target
          toTargetToolchain = target: fpkgs.targets.${target}.${channel}.rust-std;

        in fpkgs.combine (
          # Combine components from channel with toolchains for additional targets
          lib.attrVals components fpkgs.${channel} ++ map toTargetToolchain targets
        );

      # Create developer shell for combination of build and target package set
      mkDevShell = pkgs: tpkgs: pkgs.mkShell {
        # Provide target platform to cargo via env var
        CARGO_BUILD_TARGET = tpkgs.stdenv.targetPlatform.config;

        # Provide needed build tools:
        nativeBuildInputs = [
          # - Customized rust toolchain (via path)
          (mkRustToolchain tpkgs)
        ];

        # - Protobuf compiler (via env var)
        PROTOC = "${pkgs.protobuf}/bin/protoc";

        # Provide needed dependencies:
        buildInputs = [
          tpkgs.zlib.static
        ];
      };
      
      # Create developer shell for combination of build package set and target abi
      mkAbiShell = pkgs: abi: mkDevShell pkgs (mkCrossPkgs pkgs abi);
    in {
      # For every supported system ...
      devShells = lib.genAttrs supportedSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in {
          # ... provide default environment for local target ...
          default = mkDevShell pkgs pkgs;
          # ... and one variant for each supported abi target
        } // lib.genAttrs supportedAbis (mkAbiShell pkgs)
      );
    };
}
