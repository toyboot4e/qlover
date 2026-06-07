{
  description = "A stenography engine in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs =
    {
      self,
      nixpkgs,
      treefmt-nix,
      ...
    }:
    let
      forAllSystems = nixpkgs.lib.genAttrs nixpkgs.lib.systems.flakeExposed;

      pkgsFor = forAllSystems (system: import nixpkgs { inherit system; });

      treefmtFor = forAllSystems (
        system:
        treefmt-nix.lib.evalModule pkgsFor.${system} {
          projectRootFile = "flake.nix";
          programs = {
            nixfmt.enable = true;
            rustfmt.enable = true;
          };
        }
      );

      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);

      buildInputsFor = forAllSystems (
        system:
        let
          pkgs = pkgsFor.${system};
        in
        [
          pkgs.udev
          pkgs.libevdev
        ]
        ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
          pkgs.xorg.libX11
          pkgs.xorg.libXtst
        ]
      );

      qloverFor = forAllSystems (
        system:
        let
          pkgs = pkgsFor.${system};
        in
        pkgs.rustPlatform.buildRustPackage {
          pname = cargoToml.package.name;
          version = cargoToml.package.version;
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = buildInputsFor.${system};
          # Tests need HID devices / X server.
          doCheck = false;
        }
      );
    in
    {
      packages = forAllSystems (system: {
        default = qloverFor.${system};
        qlover = qloverFor.${system};
        treefmt = treefmtFor.${system}.config.build.wrapper;
      });

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${qloverFor.${system}}/bin/qlover";
        };
      });

      devShells = forAllSystems (system: {
        default = pkgsFor.${system}.mkShell {
          inputsFrom = [ qloverFor.${system} ];
          LD_LIBRARY_PATH = pkgsFor.${system}.lib.makeLibraryPath buildInputsFor.${system};
        };
      });

      formatter = forAllSystems (system: treefmtFor.${system}.config.build.wrapper);

      checks = forAllSystems (system: {
        treefmt = treefmtFor.${system}.config.build.check self;
      });
    };
}
