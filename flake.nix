{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
  };

  outputs = { nixpkgs, ... }:
    let
      inherit (nixpkgs) lib;

      makePackages = (system: dev:
        let
          pkgs = import nixpkgs {
            inherit system;
          };
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            name = "raw-video-player";
            src = lib.cleanSourceWith {
              src = ./.;
              filter = path: type:
                lib.cleanSourceFilter path type
                && (
                  let
                    relPath = lib.removePrefix (builtins.toString ./.) (builtins.toString path);
                  in
                  lib.any (re: builtins.match re relPath != null) [
                    "/Cargo.toml"
                    "/Cargo.lock"
                    "/\.cargo"
                    "/\.cargo/.*"
                    "/src"
                    "/src/.*"
                  ]
                );
            };

            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = { };
            };

            nativeBuildInputs = with pkgs; [
              pkg-config
            ] ++ (with pkgs.gst_all_1; [
              gstreamer.bin
            ]) ++ (if dev then
              with pkgs; [
                clippy
                (rustfmt.override { asNightly = true; })
                rust-analyzer
                yt-dlp
                ffmpeg
                graphviz
              ] else [ ]);

            buildInputs = with pkgs; [
              glib
              openssl
              curl
              cacert
            ] ++ (with pkgs.gst_all_1; [
              gstreamer
              gst-plugins-base
              gst-plugins-good
              gst-plugins-bad
              gst-rtsp-server
              gst-editing-services
              # gst-vaapi # linux doesn't like vp9 rtsp through vaapi??
              gst-libav
              libsoup
            ]);
          };
        }
      );
    in
    builtins.foldl' lib.recursiveUpdate { } (builtins.map
      (system: {
        devShells.${system} = makePackages system true;
        packages.${system} = makePackages system false;
      })
      lib.systems.flakeExposed);
}
