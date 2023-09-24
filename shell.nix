{ pkgs ? import <nixpkgs> { } }:
let
  todoScript = pkgs.writeScriptBin "todo-rs" ''
    cargo run TODO
  '';
in pkgs.mkShell {
  name = "todo-rs";
  buildInputs = [ pkgs.cargo pkgs.rustc pkgs.glibc pkgs.ncurses ];
  
  shellHook = ''
    cargo run TODO
  '';
}
