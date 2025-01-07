let
  pkgs = import ("https://github.com/NixOS/nixpkgs/de1864217bfa9b5845f465e771e0ecb48b30e02d.tar.gz") {};

in pkgs.mkShell {
  buildInputs = [ pkgs.cargo pkgs.rustc ];
}
