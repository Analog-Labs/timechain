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

          # Read details from toolchain file
          toml = fromTOML (builtins.readFile ./rust-toolchain.toml);

          # Download associated rust toolchain from mozilla
          toolchain = fpkgs.fromToolchainName {
            name = toml.toolchain.channel;
            sha256 = "opUgs6ckUQCyDxcB9Wy51pqhd0MPGHUVbwRKKPGiwZU=";
          };

          # Determine profile or use default
          profile = toml.toolchain.profile or "default";

          # Collect profile components and any components additional from toolchain file
          components = lib.unique (toolchain.manifest.profiles.${profile} ++ toml.toolchain.components or []);

          # List of all components actually available in toolchain
          tools = lib.attrVals (builtins.filter (c: toolchain ? ${c}) components) toolchain;

          # Additionally add toolchain for cross chain target and any from the toolchain file
          targets = [ (tpkgs.rust.toRustTarget tpkgs.stdenv.targetPlatform) ] ++ toml.toolchain.targets or [];

          # Helper function to retrieve toolchain for each target
          toTargetToolchain = target: (fpkgs.targets.${target}.fromManifest toolchain.manifest).rust-std;

        in fpkgs.combine (
          # Combine components from channel with toolchains for additional targets
          tools ++ map toTargetToolchain targets
        );

      # Create developer shell for combination of build and target package set
      mkDevShell = pkgs: tpkgs: pkgs.mkShell {
        # Provide target platform to cargo via env var
        CARGO_BUILD_TARGET = tpkgs.rust.toRustTarget tpkgs.stdenv.targetPlatform;

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
