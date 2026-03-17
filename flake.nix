{
  description = "Chords - A native GNOME guitar chords viewer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "chords";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = with pkgs; [
            pkg-config
            glib
            wrapGAppsHook4
          ];

          buildInputs = with pkgs; [
            gtk4
            libadwaita
            sqlite
            openssl
          ];

          postInstall = ''
            mkdir -p $out/share/glib-2.0/schemas
            cp data/de.chords.Chords.gschema.xml $out/share/glib-2.0/schemas/
            glib-compile-schemas $out/share/glib-2.0/schemas/

            mkdir -p $out/share/applications
            cat > $out/share/applications/de.chords.Chords.desktop << EOF
            [Desktop Entry]
            Name=Chords
            Comment=Guitar chords viewer
            Exec=chords
            Icon=emblem-music-symbolic
            Terminal=false
            Type=Application
            Categories=Audio;Music;
            EOF
          '';

          meta = with pkgs.lib; {
            description = "A native GNOME guitar chords viewer";
            homepage = "https://github.com/bjesus/chords";
            license = licenses.gpl3Only;
            mainProgram = "chords";
          };
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.default ];
          packages = with pkgs; [ rust-analyzer clippy ];
        };
      });
}
