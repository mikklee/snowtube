{
  description = "ytrs - YouTube client in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        yt-dlp-git = pkgs.yt-dlp.overrideAttrs (old: {
          version = "2026.03.03";
          src = pkgs.fetchFromGitHub {
            owner = "yt-dlp";
            repo = "yt-dlp";
            rev = "2026.03.03";
            sha256 = "sha256-BPZzMT1IrZvgva/m5tYMaDYoUaP3VmpmcYeOUOwuoUY=";
          };
          patches = [ ];
          postPatch = "";
        });


        isDarwin = pkgs.stdenv.isDarwin;

        # GStreamer packages - only used on Linux due to nixpkgs packaging issues on macOS
        gstPkgs = pkgs.gst_all_1;

        gstreamerPlugins = [
          pkgs.glib
          pkgs.glib-networking
          gstPkgs.gstreamer
          gstPkgs.gst-plugins-base
          gstPkgs.gst-plugins-good
          gstPkgs.gst-plugins-bad
        ];

        linuxDeps = with pkgs; [
          gcc
          libGL
          libxkbcommon
          wayland
        ];

        # On macOS, skip nix GStreamer - use Homebrew instead (brew install gstreamer)
        commonDeps =
          with pkgs;
          [
            rustup
            pkg-config
            mpv
            yt-dlp-git
            nil
            nixd
            deno # yt-dlp needs this to solve challenges
          ]
          ++ (if isDarwin then [ ] else gstreamerPlugins);

        gstPluginPath = pkgs.lib.makeSearchPath "lib/gstreamer-1.0" gstreamerPlugins;

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = commonDeps ++ (if isDarwin then [ ] else linuxDeps);

          shellHook = ''
            export GIO_EXTRA_MODULES="${pkgs.glib-networking}/lib/gio/modules"
            ${
              if isDarwin then
                ''
                  # On macOS, use Homebrew GStreamer to avoid nix packaging issues
                  # Install with: brew install gstreamer
                  if command -v brew &> /dev/null; then
                    BREW_PREFIX="$(brew --prefix)"
                    export GST_PLUGIN_SYSTEM_PATH_1_0="$BREW_PREFIX/lib/gstreamer-1.0"
                    # Blacklist problematic validate plugins
                    export GST_PLUGIN_FEATURE_RANK="validatessim:0"
                    # Use PKG_CONFIG_LIBDIR to override nix's pkg-config search paths entirely
                    export PKG_CONFIG_LIBDIR="$BREW_PREFIX/lib/pkgconfig"
                    unset PKG_CONFIG_PATH
                    unset PKG_CONFIG_PATH_FOR_TARGET
                    export DYLD_LIBRARY_PATH="$BREW_PREFIX/lib:$DYLD_LIBRARY_PATH"
                  else
                    echo ""
                    echo "ERROR: GStreamer not available via nix on macOS due to packaging issues."
                    echo "Please install GStreamer via Homebrew:"
                    echo ""
                    echo "  brew install gstreamer"
                    echo ""
                  fi
                ''
              else
                ''
                  # Clear any stale gstreamer cache
                  rm -rf ~/.cache/gstreamer-1.0 2>/dev/null || true

                  export GST_PLUGIN_SYSTEM_PATH_1_0=""
                  export GST_PLUGIN_PATH_1_0="${gstPluginPath}"
                  export LD_LIBRARY_PATH="${
                    pkgs.lib.makeLibraryPath (linuxDeps ++ gstreamerPlugins)
                  }:$LD_LIBRARY_PATH"
                  export RUST_SRC_PATH="${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}"
                  export OPENSSL_DIR="${pkgs.openssl.dev}"
                  export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
                  export PATH="$HOME/.cargo/bin:$PATH"
                  export RUST_LOG="debug"
                  export GIO_EXTRA_MODULES="${pkgs.glib-networking}/lib/gio/modules:$GIO_EXTRA_MODULES"
                ''
            }
            rustup default stable 2>/dev/null || true
          '';
        };
      }
    );
}
