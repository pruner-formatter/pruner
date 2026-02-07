{}: let
  a = ''/root'';
  embeddedTs =
    # typescript
    ''
      import {  spawn  } from "bun";

      const proc = spawn(["${a}/bin/foo", ...args], {
        stdio: ["inherit", "inherit", "inherit"],
      });
    '';
