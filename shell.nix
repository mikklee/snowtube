{ pkgs ? import <nixpkgs> { } }:
let
  libPath = with pkgs; lib.makeLibraryPath [
    libGL
    libxkbcommon
    wayland
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
    ];

    RUST_LOG = "debug";
    LD_LIBRARY_PATH = libPath;
    OPENSSL_DIR = "${pkgs.openssl.dev}";
    OPENSSL_LIB_DIR = "${openssl.out}/lib";

    shellHook = ''
      export PATH="$HOME/.cargo/bin:$PATH"
    '';
  };
}
