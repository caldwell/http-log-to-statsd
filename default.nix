{ # nixpkgs libs:
  stdenv, lib, callPackage, buildRustCrate, defaultCrateOverrides, darwin,
  # third party deps:
  crate2nix,
  # Extra params:
  my-version }:

let
  # We have to reach into crate2nix source and call the tools.nix code directly. If we try to use it as
  # a flake we'd have to pick a "system" and we don't have one here. Instead we call their function
  # directly with our "pkgs" which ends up being one of our overlay parameters (and which has an implied
  # system).
  cargoNixGen = (callPackage "${crate2nix}/tools.nix" {}).generatedCargoNix {
    name = "http-log-to-statsd";
    src = with lib.fileset; toSource {
      root = ./.;
      fileset = (intersection (gitTracked ./.) (unions [ ./Cargo.toml ./Cargo.lock ./http-log-to-statsd.rs ]));
    };
  };
  cargoNix = callPackage "${cargoNixGen}/default.nix" {
    buildRustCrateForPkgs = pkgs: pkgs.buildRustCrate.override {
      defaultCrateOverrides = pkgs.defaultCrateOverrides // {
        http-log-to-statsd = attrs: {
          # Don't know why we need this, but without it there's a link error (and I saw this in other people's examples).
          buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.darwin.apple_sdk.frameworks.Security ];
        };
      };
    };
  };

in {
  http-log-to-statsd = cargoNix.rootCrate.build;
}
