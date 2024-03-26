{
  description = "Http Log To Statsd";

  inputs = {
    nixpkgs.url = "nixpkgs";
    crate2nix.url = "github:nix-community/crate2nix";
    crate2nix.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = args @ { self, nixpkgs, crate2nix, flake-utils }:
    let
      my-version = "${self.ref or self.rev or self.dirtyRev or "dirty"}-${self.lastModifiedDate}";
      package = import ./default.nix;
    in
      (flake-utils.lib.eachDefaultSystem (system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
          with (pkgs.callPackage (import ./default.nix) { inherit my-version crate2nix; }); rec {
            packages = {
              inherit http-log-to-statsd;
              default = http-log-to-statsd;
            };
            apps.default    = { type = "app"; program = "${packages.http-log-to-statsd}/bin/http-log-to-statsd"; };
          }
      )) // {
        # Now we can add in the nixosModules, which don't have a $system. We do this by using the built-in nixOS overlay stuff.
        nixosModules.http-log-to-statsd = args@{ pkgs, lib, config, ...}: (import ./module.nix) (args // { inherit my-version crate2nix; });
      };
}
