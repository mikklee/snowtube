{ pkgs ? import <nixpkgs> { } }:
let
  libPath = with pkgs; lib.makeLibraryPath [
    libGL
    libxkbcommon
    wayland
    gst_all_1.gstreamer
    gst_all_1.gst-plugins-base
    gst_all_1.gst-plugins-good
    gst_all_1.gst-plugins-bad
    gst_all_1.gst-plugins-ugly
  ];
in
{
  devShell = with pkgs; mkShell {
    buildInputs = [
      rustup
      rust-analyzer
      gcc
      nil
      nixd
      pkg-config
      glib
      gst_all_1.gstreamer
      gst_all_1.gst-plugins-base
      gst_all_1.gst-plugins-good
      gst_all_1.gst-plugins-bad
      gst_all_1.gst-plugins-ugly
    ];

    RUST_LOG = "debug";
    LD_LIBRARY_PATH = libPath;
    GST_PLUGIN_PATH = lib.makeSearchPath "lib/gstreamer-1.0" [
      gst_all_1.gst-plugins-base
      gst_all_1.gst-plugins-good
      gst_all_1.gst-plugins-bad
      gst_all_1.gst-plugins-ugly
    ];
    OPENSSL_DIR = "${pkgs.openssl.dev}";
    OPENSSL_LIB_DIR = "${openssl.out}/lib";

    shellHook = ''
      export PATH="$HOME/.cargo/bin:$PATH"
    '';
  };
}
