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
          cargo-edit

          pkg-config
          makeWrapper

          alsa-lib
          alsa-utils
          pulseaudio
          pipewire
          jack2

          libGL
          mesa
          libglvnd
          vulkan-loader
          vulkan-headers
          vulkan-tools
          libdrm
          libgbm
          libxcb
          fontconfig
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

            LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath packages}:/run/opengl-driver/lib";
            PKG_CONFIG_PATH = "${pkgs.alsa-lib.dev}/lib/pkgconfig:${pkgs.jack2.dev}/lib/pkgconfig";
            WGPU_BACKEND = "vulkan";

            shellHook = ''
              if [ -d /run/opengl-driver/share/vulkan/icd.d ]; then
                export VK_DRIVER_FILES="$(find /run/opengl-driver/share/vulkan/icd.d -name '*_icd.*.json' 2>/dev/null | paste -sd: -)"
              fi

              if [ -z "''${VK_DRIVER_FILES:-}" ]; then
                export VK_DRIVER_FILES="${pkgs.mesa}/share/vulkan/icd.d/lvp_icd.x86_64.json"
              fi

              export VK_ICD_FILENAMES="$VK_DRIVER_FILES"
            '';
          };
        };
        packages.default = pkgs.callPackage ./package.nix { };
      }
    );
}
