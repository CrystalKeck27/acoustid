{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = { self, nixpkgs }: let
    pkgs = nixpkgs.legacyPackages."x86_64-linux"; 
    mkRustShell = buildInputs: pkgs.mkShell {
      inherit buildInputs;
      shellHook = ''
        export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${builtins.toString (pkgs.lib.makeLibraryPath buildInputs)}";
      '';
      env.RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
    };
    developer = mkRustShell (with pkgs; [
      cargo
      rustc
      rustfmt
      clippy
      rust-analyzer
      release-plz
    ]);
    ci = mkRustShell (with pkgs; [
      cargo
      rustc
      rustfmt
      clippy
    ]);
  in {
    devShells."x86_64-linux" = {
      default = developer;
      inherit developer ci;
    };
  };
}
