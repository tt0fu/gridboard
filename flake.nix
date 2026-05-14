{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        packages = with pkgs; [
          rustc
          cargo
          rustfmt
          rust-analyzer
          gdb
          cargo-expand

          pkg-config
          makeWrapper

          alsa-lib
          alsa-utils
          pulseaudio
          pipewire
          jack2

          libxkbcommon
          wayland
          wayland.dev
          libx11
          libxcursor
          libxi
          libxrandr
        ];
      in
      {
        devShells = {
          default = pkgs.mkShell {
            buildInputs = packages;

            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath packages;
            PKG_CONFIG_PATH = "${pkgs.alsa-lib.dev}/lib/pkgconfig:${pkgs.jack2.dev}/lib/pkgconfig";
          };
        };
        # packages = {
        #   default = pkgs.callPackage ./package.nix { };
        #   portable = pkgs.callPackage ./package.nix { portable = true; };
        # };
      }
    );
}
