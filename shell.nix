{ pkgs ? import <nixpkgs> {} }:
  let
    libPath = with pkgs; lib.makeLibraryPath [
      libGL
      libxkbcommon
      wayland
    ];
  in {
    devShell = with pkgs; mkShell {
      buildInputs = [
        cargo
        rustc
        rust-analyzer
        gcc
        rustfmt
        clippy
        nil
        nixd
      ];

      RUST_LOG = "debug";
      RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
      LD_LIBRARY_PATH = libPath;
      OPENSSL_DIR= "${pkgs.openssl.dev}";
      OPENSSL_LIB_DIR= "${openssl.out}/lib";
    };
  }
