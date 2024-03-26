# NixOS module (ie, how to configure it on NixOS)

{ pkgs, lib, config, my-version, crate2nix, ...}:
let
  inherit (lib) types mkIf mkOption optionalAttrs;

  cfg = config.services.http-log-to-statsd;
in {
  options.services.http-log-to-statsd = {
    enable = lib.mkEnableOption (lib.mdDoc "Read specialized custom logs from nginx/apache via UDP and write the data out to a statsd server.");

    listen = mkOption {
      type = types.str;
      default = "127.0.0.1:6666";
      description = "Host and port to listen on.";
    };

    statsd-server = mkOption {
      type = types.str;
      default = "127.0.0.1:8125";
      description = "Host and port of the statsd server.";
    };

    prefix = mkOption {
      type = types.str;
      default = "http.request";
      description = "Statsd metrics prefix.";
    };

    user = mkOption {
      type = types.str;
      default = "http-log-to-statsd";
      description = lib.mdDoc "User under which http-log-to-statsd is ran.";
    };

    group = mkOption {
      type = types.str;
      default = "http-log-to-statsd";
      description = lib.mdDoc "Group under which http-log-to-statsd is ran.";
    };
  };

  config = mkIf cfg.enable {
    nixpkgs.overlays = [
      (final: prev: { inherit (final.callPackage (import ./default.nix) { inherit my-version crate2nix; }) http-log-to-statsd; })
    ];

    users = {
      users = optionalAttrs (cfg.user == "http-log-to-statsd") {
        http-log-to-statsd = {
          group = cfg.group;
          home = "/var/empty";
          isSystemUser = true;
        };
      };
      groups = optionalAttrs (cfg.group == "http-log-to-statsd") { http-log-to-statsd = { }; };
    };

    systemd.services.http-log-to-statsd = {
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        Type = "simple";
        User = cfg.user;
        Group = cfg.group;
        Restart = "always";
        RestartSec = 1;
      };

      path = [ pkgs.http-log-to-statsd ];
      script = ''
        http-log-to-statsd --listen="${cfg.listen}" --statsd="${cfg.statsd-server}" --prefix="${cfg.prefix}"
      '';
    };
  };
}
