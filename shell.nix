{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    pkgs.cargo
    pkgs.rustc
  ];

  shellHook = ''
    rustc --version
    cargo --version

    export PATH="/usr/bin/:$PATH"
  '';
}
