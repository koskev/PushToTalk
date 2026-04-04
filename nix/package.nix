{ inputs, self, ... }:
{
  perSystem =
    {
      pkgs,
      ...
    }:
    let
      craneLib = inputs.crane.mkLib pkgs;
    in
    {
      packages.default = craneLib.buildPackage {
        src = self;

        nativeBuildInputs = with pkgs; [
          makeWrapper
        ];

        postInstall = ''
          wrapProgram $out/bin/push_to_talk_rs \
          --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.pulseaudio ]}
        '';
      };
    };
}
