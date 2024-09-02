{
  description = "A build manager similar to make";
  inputs = {
    nixpkgs.url = "nixpkgs";
    cf.url = "github:jzbor/cornflakes";
    cf.inputs.nixpkgs.follows = "nixpkgs";
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, cf, crane }:
  cf.lib.flakeForDefaultSystems (system:
  let
    craneLib = crane.mkLib nixpkgs.legacyPackages.${system};
  in {
    ### PACKAGES ###
    packages = {
      default = craneLib.buildPackage {
        pname = "zinn";

        src = ./.;

        # Add extra inputs here or any other derivation settings
        # doCheck = true;
      };
    };
  }) // {
    ### OVERLAY ###
    overlays.default = _: prev: {
      zinn = self.packages.${prev.system}.default;
    };
  };
}

