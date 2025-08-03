{
  description = "An API for browsing snapshots";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, ... }:
    let
      system = "x86_64-linux";

      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };

      rust-bin-custom = pkgs.rust-bin.stable.latest.default.override {
        extensions = [ "rust-src" "rust-analyzer" ];
        targets = [ "x86_64-unknown-linux-gnu" ];
      };

      snapshot-browser-cargo-toml = (builtins.fromTOML (builtins.readFile ./Cargo.toml));
      hashes-toml = (builtins.fromTOML (builtins.readFile ./hashes.toml));

      snapshot-browser-deps = derivation {
        inherit system;
        name = "${snapshot-browser-cargo-toml.package.name}-${hashes-toml.cargo_lock}-deps";
        builder = "${pkgs.nushell}/bin/nu";
        buildInputs = with pkgs; [
          rust-bin-custom
        ];
        args = [ ./builder.nu "vendor" ./. ];

        outputHashAlgo = "sha256";
        outputHashMode = "recursive";
        outputHash = hashes-toml.deps;
        # outputHash = pkgs.lib.fakeHash;
      };

      snapshot-browser-bin = derivation {
          inherit system;
          name = "${snapshot-browser-cargo-toml.package.name}-v${snapshot-browser-cargo-toml.package.version}";
          builder = "${pkgs.nushell}/bin/nu";
          buildInputs = with pkgs; [
            gcc_multi
            rust-bin-custom
          ];
          args = [ ./builder.nu "build" ./. snapshot-browser-deps "snapshot-browser" hashes-toml.cargo_config ];
      };
    in {
      packages.${system} = {
        deps = snapshot-browser-deps;
        bin = snapshot-browser-bin;
        default = snapshot-browser-bin;
      };

      nixosModules.${system}.default = { config, lib, pkgs, ... }:
        with lib;
        let
          cfg = config.hochreiner.services.snapshot-browser;
    
          snapshotRoot = {
            options = {
              path = mkOption {
                type = types.path;
                description = lib.mdDoc "Path to the snapshot root directory";
              };
              suffix = mkOption {
                type = types.str;
                default = "";
                description = lib.mdDoc "Suffix for the snapshot root (e.g. '-snapshots')";
              };
            };
          };

          configuration_file = pkgs.writeTextFile "snapshot-browser-config" (builtins.toJSON cfg.configuration);
        in {
          # https://britter.dev/blog/2025/01/09/nixos-modules/
          options.hochreiner.services.snapshot-browser = {
            enable = mkEnableOption "Enables the snapshot-browser service";

            configuration.snapshot_roots = mkOption {
              type = types.attrsOf snapshotRoot;
              default = {};
              description = lib.mdDoc "Snapshot roots configuration";
            };

            log_level = mkOption {
              type = types.enum [ "error" "warn" "info" "debug" "trace" ];
              default = "info";
              description = lib.mdDoc "Log level";
            };

            port = mkOption {
              type = types.port;
              default = 8080;
              description = lib.mdDoc "Port to run the snapshot-browser service on";
            };

            address = mkOption {
              type = types.str;
              default = "0.0.0.0";
              description = lib.mdDoc "Address to bind the snapshot-browser service to";
            };
          };

          config = mkIf cfg.enable {
            systemd.services."hochreiner.snapshot-browser" = {
              description = "snapshot-browser service";
              serviceConfig = let pkg = self.packages.${system}.default;
              in {
                Type = "oneshot";
                ExecStart = "${pkg}/bin/snapshot-browser";
                Environment = "RUST_LOG='${cfg.log_level}' SNAPSHOT_BROWSER_CONFIG='${configuration_file}' PATH=/run/current-system/sw/bin";
              };
            };
          };
        };

      devShells.${system}.default = pkgs.mkShell {
        name = "snapshot-browser";

        shellHook = ''
          exec nu
        '';
        # Additional dev-shell environment variables can be set directly
        # MY_CUSTOM_DEVELOPMENT_VAR = "something else"
        # Extra inputs can be added here; cargo and rustc are provided by default.
        buildInputs = with pkgs; [
          rust-bin-custom
        ];
      };
    };
  
  nixConfig = {
    substituters = [
      "https://cache.nixos.org"
      "https://hannes-hochreiner.cachix.org"
    ];
    trusted-public-keys = [
      "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
      "hannes-hochreiner.cachix.org-1:+ljzSuDIM6I+FbA0mdBTSGHcKOcEZSECEtYIEcDA4Hg="
    ];
  };
}