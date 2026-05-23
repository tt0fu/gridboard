{
  lib,
  rustPlatform,
  pkg-config,
  alsa-lib,
  alsa-utils,
  pulseaudio,
  pipewire,
  jack2,
  libGL,
  fontconfig,
  libxkbcommon,
  wayland,
  libx11,
  libxcursor,
  libxi,
  libxrandr,
  makeDesktopItem,
  ...
}:
let
  name = "gridboard";
  desktopItem = makeDesktopItem {
    inherit name;
    desktopName = name;
    exec = name;
  };
in
rustPlatform.buildRustPackage rec {
  pname = name;
  version = (fromTOML (builtins.readFile ./Cargo.toml)).package.version;
  doCheck = false;
  cargoLock = {
    lockFile = ./Cargo.lock;
  };
  src = ./.;

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    alsa-lib
    alsa-utils
    pulseaudio
    pipewire
    jack2

    libGL
    fontconfig
    libxkbcommon
    wayland
    wayland.dev
    libx11
    libxcursor
    libxi
    libxrandr
  ];

  env = {
    PKG_CONFIG_PATH = "${alsa-lib.dev}/lib/pkgconfig:${jack2.dev}/lib/pkgconfig";
  };

  postFixup = ''
    patchelf \
      --set-rpath ${lib.makeLibraryPath buildInputs} \
      $out/bin/${name}
  '';

  postInstall = ''
    install -Dm444 ${desktopItem}/share/applications/*.desktop -t $out/share/applications
  '';

  passthru = {
    desktopItems = [ desktopItem ];
  };

  meta = {
    description = "A simple soundboard";
    mainProgram = name;
    license = lib.licenses.gpl3;
  };
}
