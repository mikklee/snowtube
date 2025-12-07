{
  pkgs ? import <nixpkgs> { },
}:
let
  libPath =
    with pkgs;
    lib.makeLibraryPath [
      libGL
      libxkbcommon
      wayland
      gst_all_1.gstreamer
      gst_all_1.gst-plugins-base
      gst_all_1.gst-plugins-good
      gst_all_1.gst-plugins-bad
      gst_all_1.gst-plugins-ugly
    ];

  currentSystemIsMac = builtins.isList (builtins.match ".*darwin" (builtins.currentSystem));

  platformDeps =
    if currentSystemIsMac then
      [
        pkgs.rustup
      ]
    else
      [
        pkgs.gcc
        pkgs.cargo
        pkgs.rustc
      ];

  gstreamerDeps = with pkgs; [
    pkg-config
    glib
    glib-networking
    gst_all_1.gstreamer
    gst_all_1.gst-plugins-base
    gst_all_1.gst-plugins-good
    gst_all_1.gst-plugins-bad
    gst_all_1.gst-plugins-ugly
  ];

  env =
    if currentSystemIsMac then
      {
        shellHook = ''
          rustup update stable
          rustup default stable
          export GIO_EXTRA_MODULES="${pkgs.glib-networking}/lib/gio/modules:$GIO_EXTRA_MODULES"
        '';
      }
    else
      {
        RUST_LOG = "debug";
        RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        LD_LIBRARY_PATH = libPath;
        GST_PLUGIN_PATH = pkgs.lib.makeSearchPath "lib/gstreamer-1.0" [
          pkgs.gst_all_1.gst-plugins-base
          pkgs.gst_all_1.gst-plugins-good
          pkgs.gst_all_1.gst-plugins-bad
          pkgs.gst_all_1.gst-plugins-ugly
        ];
        OPENSSL_DIR = "${pkgs.openssl.dev}";
        OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
        shellHook = ''
          export PATH="$HOME/.cargo/bin:$PATH"
          export GIO_EXTRA_MODULES="${pkgs.glib-networking}/lib/gio/modules:$GIO_EXTRA_MODULES"
        '';
      };

in
{
  devShell =
    with pkgs;
    mkShell (
      {
        buildInputs = [
          rust-analyzer
          rustfmt
          clippy
          nil
          nixd
          mpv
          yt-dlp
        ]
        ++ platformDeps
        ++ gstreamerDeps;
      }
      // env
    );
}
