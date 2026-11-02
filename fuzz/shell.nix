# A nightly shell for the fuzz targets and Miri: cargo-fuzz needs the `-Z`
# sanitizer flags stable rustc rejects, and Miri is a nightly component. The
# toolchain comes from the fenix revision flake.lock pins, cargo-fuzz from its
# nixpkgs. Enter it from the repository root:
#
#   nix-shell fuzz/shell.nix --run "cargo fuzz run decode"
#   nix-shell fuzz/shell.nix --run "cargo miri test"
{
  lock ? builtins.fromJSON (builtins.readFile ../flake.lock),
  locked ? name: with lock.nodes.${name}.locked; builtins.getFlake "github:${owner}/${repo}/${rev}",
  system ? builtins.currentSystem,
  pkgs ? (locked "nixpkgs").legacyPackages.${system},
  fenix ? (locked "fenix").packages.${system},
}:

pkgs.mkShell {
  packages = [
    (fenix.latest.withComponents [
      "cargo"
      "miri"
      "rust-src"
      "rustc"
    ])
    pkgs.cargo-fuzz
  ];

  # The libFuzzer binary cargo-fuzz builds links libstdc++ dynamically, which
  # NixOS has no global copy of: point the loader at the compiler's.
  shellHook = ''
    export LD_LIBRARY_PATH="${pkgs.stdenv.cc.cc.lib}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  '';
}
