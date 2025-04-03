{
  stdenv,
  lib,
  buildPythonPackage,
  fetchFromGitHub,
  pytestCheckHook,
  pythonOlder,
  rustPlatform,
  numpy
}:

buildPythonPackage rec {
  pname = "dbase";
  version = "0.1.0";
  format = "pyproject";

  disabled = pythonOlder "3.7";

  src = ../../..;

  cargoDeps = rustPlatform.fetchCargoTarball {
    inherit src;
    name = "${pname}-${version}";
    hash = "sha256-/N/p6doTwmDQGeNGW7S78tnot7OwDP07Svs7IIW3+CQ=";
  };

  nativeBuildInputs = with rustPlatform; [
    cargoSetupHook
    maturinBuildHook
  ];

  propagatedBuildInputs = [
    numpy
  ];

  pythonImportsCheck = [ "dbase" ];
}
