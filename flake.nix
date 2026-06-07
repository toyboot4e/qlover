{
  description = "A stenography engine in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      treefmt-nix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        treefmtEval = treefmt-nix.lib.evalModule pkgs {
          projectRootFile = "flake.nix";
          programs = {
            nixfmt.enable = true;
            rustfmt.enable = true;
          };
        };

        nativeBuildInputs = [ pkgs.pkg-config ];

        buildInputs =
          [
            pkgs.udev
            pkgs.libevdev
          ]
          ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.xorg.libX11
            pkgs.xorg.libXtst
          ];

        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);

        qlover = pkgs.rustPlatform.buildRustPackage {
          pname = cargoToml.package.name;
          version = cargoToml.package.version;
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          inherit nativeBuildInputs buildInputs;
          # Tests need HID devices / X server.
          doCheck = false;
        };
      in
      {
        packages = {
          default = qlover;
          qlover = qlover;
          treefmt = treefmtEval.config.build.wrapper;
        };

        apps.default = {
          type = "app";
          program = "${qlover}/bin/qlover";
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ qlover ];

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
        };

        # nix fmt
        formatter = treefmtEval.config.build.wrapper;

        checks.treefmt = treefmtEval.config.build.check self;
      }
    );
}
