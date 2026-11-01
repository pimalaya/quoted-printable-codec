# TODO: move this to nixpkgs
# This file aims to be a replacement for the nixpkgs derivation.

{
  buildFeatures ? [ ],
  buildNoDefaultFeatures ? false,
  buildPackages,
  fetchFromGitHub,
  lib,
  rustPlatform,
}:

rustPlatform.buildRustPackage {
  inherit buildNoDefaultFeatures buildFeatures;

  pname = "quoted-printable-codec";
  version = "0.0.1";
  cargoHash = "";

  src = fetchFromGitHub {
    owner = "pimalaya";
    repo = "quoted-printable-codec";
    rev = "v0.0.1";
    hash = "";
  };

  meta = {
    description = "Quoted-printable encoder and decoder, no_std and dependency-free";
    homepage = "https://github.com/pimalaya/quoted-printable-codec";
    changelog = "https://github.com/pimalaya/quoted-printable-codec/blob/master/CHANGELOG.md";
    license = [
      lib.licenses.mit
      lib.licenses.asl20
    ];
    maintainers = with lib.maintainers; [ soywod ];
  };
}
