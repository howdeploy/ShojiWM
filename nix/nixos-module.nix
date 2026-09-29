{ self }:

{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.programs.shojiwm;
  system = pkgs.stdenv.hostPlatform.system;
  defaultPackage = self.packages.${system}.default;
  defaultXwayland = pkgs.xwayland or (pkgs.xorg.xwayland or null);
  defaultSatellite = pkgs.xwayland-satellite or null;
  gtkPortal = pkgs.xdg-desktop-portal-gtk or null;
  initConfigUsers = cfg.initConfig.users;

  shojiPackage =
    if cfg.package ? override then
      cfg.package.override {
        xwayland = defaultXwayland;
        xwaylandSatellite =
          if cfg.xwaylandSatellite.enable then cfg.xwaylandSatellite.package else null;
      }
    else
      cfg.package;
in
{
  options.programs.shojiwm = {
    enable = lib.mkEnableOption "ShojiWM";

    package = lib.mkOption {
      type = lib.types.package;
      default = defaultPackage;
      defaultText = "ShojiWM flake default package";
      description = "ShojiWM package to install.";
    };

    portal.enable = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Whether to enable xdg-desktop-portal-shojiwm for screen capture.";
    };

    portal.gtkFallback = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Whether to install xdg-desktop-portal-gtk as the fallback portal backend.";
    };

    initConfig.enable = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        Whether to initialize ShojiWM TypeScript config for selected users
        during system activation. Existing src/index.tsx files are kept, while
        the shoji_wm package symlink is refreshed on every activation.
      '';
    };

    initConfig.users = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "bea4dev" ];
      description = "Users whose ~/.config/shojiwm directory should be initialized.";
    };

    xwaylandSatellite.enable = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        Whether to run a separately packaged xwayland-satellite as its own
        process instead of the one built into ShojiWM. Useful for debugging or
        for trying another xwayland-satellite branch.
      '';
    };

    xwaylandSatellite.package = lib.mkOption {
      type = lib.types.nullOr lib.types.package;
      default = defaultSatellite;
      defaultText = lib.literalExpression "pkgs.xwayland-satellite or null";
      description = ''
        xwayland-satellite package run as a separate process when
        `xwaylandSatellite.enable` is set. ShojiWM's built-in xwayland-satellite
        is used otherwise.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = !cfg.xwaylandSatellite.enable || cfg.xwaylandSatellite.package != null;
        message = ''
          programs.shojiwm.xwaylandSatellite.enable is true, but pkgs.xwayland-satellite
          is not available. Set programs.shojiwm.xwaylandSatellite.package explicitly,
          or disable xwaylandSatellite.
        '';
      }
      {
        assertion = !cfg.portal.enable || !cfg.portal.gtkFallback || gtkPortal != null;
        message = ''
          programs.shojiwm.portal.gtkFallback is true, but xdg-desktop-portal-gtk
          is not available in this nixpkgs. Disable gtkFallback or provide a newer nixpkgs.
        '';
      }
      {
        assertion = !cfg.initConfig.enable || initConfigUsers != [ ];
        message = ''
          programs.shojiwm.initConfig.enable is true, but initConfig.users is empty.
          Set programs.shojiwm.initConfig.users to the user accounts that should
          receive ~/.config/shojiwm.
        '';
      }
      {
        assertion =
          !cfg.initConfig.enable
          || lib.all (user: builtins.hasAttr user config.users.users) initConfigUsers;
        message = ''
          programs.shojiwm.initConfig.users contains a user that is not declared
          in users.users. Declare the user in users.users or remove it from the
          initConfig.users list.
        '';
      }
    ];

    environment.systemPackages =
      [
        shojiPackage
      ]
      ++ lib.optional (defaultXwayland != null) defaultXwayland
      ++ lib.optional (cfg.xwaylandSatellite.enable && cfg.xwaylandSatellite.package != null)
        cfg.xwaylandSatellite.package
      ++ lib.optional (cfg.portal.enable && cfg.portal.gtkFallback && gtkPortal != null) gtkPortal;

    services.displayManager.sessionPackages = [ shojiPackage ];

    hardware.graphics.enable = true;

    system.activationScripts.shojiwm-init-config = lib.mkIf cfg.initConfig.enable (
      lib.stringAfter [ "users" ] (
        lib.concatMapStringsSep "\n" (
          user:
          let
            userConfig = config.users.users.${user};
            home = userConfig.home or "/home/${user}";
            group = userConfig.group or "users";
            quotedUser = lib.escapeShellArg user;
            quotedGroup = lib.escapeShellArg group;
            quotedHome = lib.escapeShellArg home;
            quotedConfigHome = lib.escapeShellArg "${home}/.config";
          in
          ''
            echo "initializing ShojiWM config for ${user}"
            install -d -m 0755 -o ${quotedUser} -g ${quotedGroup} ${quotedHome}
            ${pkgs.util-linux}/bin/runuser -u ${quotedUser} -- \
              env HOME=${quotedHome} XDG_CONFIG_HOME=${quotedConfigHome} \
              ${shojiPackage}/bin/shojiwm-init-config
          ''
        ) initConfigUsers
      )
    );

    xdg.portal = lib.mkIf cfg.portal.enable {
      enable = true;
      extraPortals =
        [ shojiPackage ]
        ++ lib.optional (cfg.portal.gtkFallback && gtkPortal != null) gtkPortal;
      config.ShojiWM =
        {
          "org.freedesktop.impl.portal.ScreenCast" = [ "shojiwm" ];
        }
        // lib.optionalAttrs cfg.portal.gtkFallback {
          default = [ "gtk" ];
        };
    };

    systemd.user.services.xdg-desktop-portal-shojiwm = lib.mkIf cfg.portal.enable {
      description = "Portal service (ShojiWM implementation)";
      partOf = [ "graphical-session.target" ];
      after = [ "graphical-session.target" ];
      serviceConfig = {
        Type = "dbus";
        BusName = "org.freedesktop.impl.portal.desktop.shojiwm";
        ExecStart = "${shojiPackage}/bin/xdg-desktop-portal-shojiwm";
        Restart = "always";
        RestartSec = "500ms";
        TimeoutStopSec = "10";
      };
    };
  };
}
